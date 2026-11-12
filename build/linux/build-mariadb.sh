#!/usr/bin/env bash
# Builds MariaDB for macOS from the source release: MariaDB publishes no macOS binaries, and
# Linux and Windows use its official archives (see runtimes.rs). OpenSSL is compiled in
# (static, as for Apache): the bundled WolfSSL covers the server but not its client library,
# which then wants GnuTLS. zlib, PCRE2 and fmt are MariaDB's bundled copies, and Homebrew's
# libraries are kept out, so the result needs only macOS's own libraries.
#   build-mariadb.sh <version>   ->  out/mariadb-<version>-macos-<arch>.tar.gz
. "$(dirname "$0")/lib.sh"
VERSION="$1"
sum_var="MARIADB_SHA256_${VERSION//./_}"
[ -n "${!sum_var:-}" ] || { echo "no checksum for MariaDB $VERSION in versions.env ($sum_var)" >&2; exit 1; }
[ "$OS" = macos ] || { echo "build-mariadb.sh is for macOS; Linux uses MariaDB's own archives" >&2; exit 1; }

# A newer Bison than macOS's 2.3, and CMake, from Homebrew (build tools only).
command -v cmake >/dev/null || brew install cmake
brew list bison >/dev/null 2>&1 || brew install bison
export PATH="$(brew --prefix bison)/bin:$PATH"

# pkg-config would otherwise hand CMake Homebrew's GnuTLS for the client library, which
# CMAKE_IGNORE_PREFIX_PATH doesn't cover.
export PKG_CONFIG_PATH="" PKG_CONFIG_LIBDIR="/nonexistent"

log "MariaDB $VERSION ($OS $ARCH): OpenSSL $OPENSSL_VERSION (static)"
ssl="$WORK/deps-mariadb"
rm -rf "$ssl"
(cd "$(openssl_src)" && ./Configure --prefix="$ssl" --libdir=lib no-shared no-tests -fPIC >/dev/null && make -j"$JOBS" >/dev/null && make install_sw >/dev/null)

log "MariaDB $VERSION ($OS $ARCH): compiling"
src="$(unpack "$(fetch "https://archive.mariadb.org/mariadb-$VERSION/source/mariadb-$VERSION.tar.gz" "${!sum_var}")")"
stage="$WORK/stage-mariadb-$VERSION"
rm -rf "$stage" "$WORK/build-mariadb"
# STANDALONE is the layout of MariaDB's own archives (bin/, scripts/, share/, lib/plugin),
# which configs.rs expects. Storage engines and plugins that need extra libraries or
# toolchains (RocksDB, Mroonga, Spider, CONNECT, ColumnStore, S3, OQGraph) are left out;
# InnoDB, Aria, MyISAM and the usual plugins stay.
cmake -S "$src" -B "$WORK/build-mariadb" \
    -DCMAKE_BUILD_TYPE=RelWithDebInfo \
    -DCMAKE_INSTALL_PREFIX="$stage" \
    -DCMAKE_IGNORE_PREFIX_PATH="/opt/homebrew;/usr/local" \
    -DINSTALL_LAYOUT=STANDALONE \
    -DWITH_SSL="$ssl" -DOPENSSL_ROOT_DIR="$ssl" -DOPENSSL_USE_STATIC_LIBS=ON \
    -DWITH_ZLIB=bundled -DWITH_PCRE=bundled -DWITH_LIBFMT=bundled \
    -DWITH_UNIT_TESTS=OFF -DWITH_EMBEDDED_SERVER=OFF -DWITH_MARIABACKUP=ON \
    -DPLUGIN_ROCKSDB=NO -DPLUGIN_MROONGA=NO -DPLUGIN_SPIDER=NO -DPLUGIN_CONNECT=NO \
    -DPLUGIN_COLUMNSTORE=NO -DPLUGIN_S3=NO -DPLUGIN_OQGRAPH=NO -DPLUGIN_TOKUDB=NO \
    -DPLUGIN_PROVIDER_LZ4=NO -DPLUGIN_PROVIDER_LZO=NO -DPLUGIN_PROVIDER_SNAPPY=NO \
    -DPLUGIN_PROVIDER_BZIP2=NO -DPLUGIN_PROVIDER_LZMA=NO \
    -DPLUGIN_AUTH_GSSAPI=NO -DPLUGIN_AUTH_PAM=NO \
    >/dev/null
cmake --build "$WORK/build-mariadb" -j"$JOBS" >/dev/null
cmake --install "$WORK/build-mariadb" >/dev/null

# Tests, the embedded test suite and debug symbols aren't needed to run it.
rm -rf "$stage/mariadb-test" "$stage/mysql-test" "$stage/sql-bench" "$stage/man" "$stage/docs"
find "$stage/bin" "$stage/lib" -type f \( -perm -u+x -o -name '*.so' -o -name '*.dylib' \) -exec strip -x {} \; 2>/dev/null || true
cp "$src"/COPYING* "$src"/README* "$stage/" 2>/dev/null || true
licenses "$stage" "$WORK/openssl-$OPENSSL_VERSION" "$src/zlib" "$src/libmariadb"
check_deps "$stage/bin/mariadbd" "$stage/bin/mariadb" "$stage/bin/mariadb-dump" "$stage/bin/mariadb-admin"
"$stage/bin/mariadbd" --version
# A first database must initialise from the archive's own layout, as LampRabit does it.
tmp="$(mktemp -d)"
"$stage/scripts/mariadb-install-db" --no-defaults --basedir="$stage" --datadir="$tmp/data" --auth-root-authentication-method=normal >/dev/null
rm -rf "$tmp"
package mariadb "$VERSION" "$stage"

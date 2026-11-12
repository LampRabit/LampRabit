/**
 * Plain words for the errors people hit most, from git, the services and the OS. Raw
 * messages ("Permission denied (publickey)", "bind: address already in use") say what
 * broke, not what to do; the hint does. Null when there's nothing better to say.
 */
const rules: [RegExp, string][] = [
  [/Permission denied \(publickey/i, "The remote didn't accept your SSH key. Add your public key to your account on the Git host, or switch the remote to HTTPS (Git tab → Remotes → Use HTTPS)."],
  [/Host key verification failed/i, "SSH doesn't know this server yet. Connect once from a terminal (ssh -T git@<host>) to accept its key, or use HTTPS."],
  [/(Authentication failed|could not read Username|terminal prompts disabled|Invalid username or password|403)/i, "The Git host needs you to sign in. Use a credential helper (Git Credential Manager on Windows), a personal access token, or switch the remote to SSH."],
  [/Repository not found|does not appear to be a git repository/i, "Check the remote's URL (Git tab → Remotes). For a private repository, your account needs access to it."],
  [/Could not resolve host|Temporary failure in name resolution|Network is unreachable|timed out/i, "LampRabit couldn't reach the server. Check your internet connection or VPN, then try again."],
  [/\[rejected\].*\(fetch first\)|non-fast-forward|Updates were rejected/i, "The remote has commits you don't have yet. Pull first, then push again."],
  [/Your local changes .* would be overwritten|Please commit your changes or stash them/i, "You have uncommitted changes in the way. Commit or stash them (Git tab → Stash), then try again."],
  [/not fully merged/i, "That branch has commits that exist nowhere else. Merge it first, or delete it anyway if you don't need them."],
  [/(address already in use|port .* (is )?(in use|taken)|only one usage of each socket address|EADDRINUSE)/i, "Another program uses that port. Stop it (Home shows who holds it), or pick another port in Settings → Ports."],
  [/(Access is denied|Permission denied|EACCES|Operation not permitted)/i, "The system refused access. Close programs that may hold the file (an editor, a terminal in that folder), or check the folder's permissions."],
  [/(No space left on device|There is not enough space on the disk)/i, "The disk is full. Free some space, or move the LampRabit folder to another drive (Settings → General)."],
  [/Git isn't installed/i, "Install Git from Services → Essentials (Developer tools → Git)."],
];

export function explain(message: string): string | null {
  for (const [re, hint] of rules) if (re.test(message)) return hint;
  return null;
}

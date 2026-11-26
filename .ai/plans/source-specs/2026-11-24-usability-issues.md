# Usability and flow issues (the user, 2026-11-24, verbatim)

1. Give option to select between any developer tool to open the database or any other service, like user can set there default postgres client to see the databases.
2. Apache doesn't get any option to Kill running port, it shows in setting and the suggested command shows on hover of running port text.
3. Changing port and saving setting still shows the error alert in the home servcie tile. restarting all services make it disappear.
4. For killing specific ports that needs admin permissoions, prompt user for this to elevate admin permissions and kill that port.
5. Every port conflict should show clearly on home service tiles.
6. Remember every service that opens in the browser should have a .test domain or the domain name the user configure for.
7. pgadmin isn't open in the browser.
8. System default NVM sometimes conflicts and won't able to use that properly, make sure everything works.
9. App should be real-time and reflect every change on the app right after the thing changes. currently needs restart for every change.
10. Let the user use the sub-domains publically via cloudflare tunnels.
11. Use self singing way to add signature in the LampRabit.

## Second list, later the same day (verbatim)

The user first said: "I've asked claude to do something into a wrong window, So he did some of the change, check them too and here are few things i've notices apart from that."

1. Services that are interdependent on each other should indicate to either start, install them before using it.
2. Add a self sign signature for this app, this may prevent the MacOS for showing untrusted and windows also prevent smart screen sometimes.
3. pgadmin isn't open in the browser.
4. I've asked him to add option to select a default viewing app for database like pgadmin, phppgadmin, Karotx or other. default should be KarotX for all. show this setting in toe services page inside all databases and database clients too.
5. Npm projects should offer different package managers, python should offer to create project with conda uv or pip. Also check for other languages too if there are multiple package managers to offer using different package manager while creating project.

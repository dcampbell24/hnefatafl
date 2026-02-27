#! /bin/bash -ex

INDEX="Join a friendly community and play Copenhagen Hnefatafl. Figure out how to \
install the software, play by the rules, chat on Discord, find Help, and Donate."

CANONICAL="<!-- Custom HTML head -->\n        <link rel=\"canonical\" href=\"https:\/\/hnefatafl.org\" \/>"

HISTORY="Get the history of Hnefatafl. It is a part of the games known Tafl games. \
Other related games are Alea evangelii, Ard Rí, Brandubh, Tablut, and Tawlbwrdd"

INSTALL="Determine how to install Copenhagen Hnefatafl. Install using the Arch User \
Repository, Chocolatey, a Debian package, a flathub package, or Rust's cargo."

RULES="Learn the rules to the game of Copenhagen Hnefatafl. Move your pieces until \
you achieve victory or lose. Try not to get surrounded as the defenders and escape."

TOURNAMENT="Learn how tournaments work. The tournament is a combination of round robin \
and single elimination with each player playing both the attacker and the defender."

AI="Discover about using artificial intelligence to play the game of Copenhagen \
Hnefatafl. If you are using the Debian or Arch installs, you can run AI as a service."

sed -i "s/{{description}}/$INDEX/" /var/www/html/index.html # book/index.html
sed -i "s/<!-- Custom HTML head -->/$CANONICAL/" /var/www/html/index.html # book/index.html

sed -i "s/{{description}}/$HISTORY/" /var/www/html/history.html # book/history.html
sed -i "s/{{description}}/$INSTALL/" /var/www/html/install.html # book/install.html
sed -i "s/{{description}}/$RULES/" /var/www/html/rules.html # book/rules.html
sed -i "s/{{description}}/$TOURNAMENT/" /var/www/html/tournaments.html # book/tournaments.html
sed -i "s/{{description}}/$AI/" /var/www/html/ai.html # book/ai.html

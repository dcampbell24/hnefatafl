#! /bin/bash -ex

mdbook build --dest-dir /var/www/html/

sed --in-place 's/\<meta name=\"description\" content=\".*\"\>/<meta name=\"description\" content=\"Determine how to install Copenhagen Hnefatafl\. Install using the Arch User Repository, Chocolatey, a Debian package (\.deb), a flathub package, or Rust\'s cargo\.\"\>/' /var/www/html/install.html
sed --in-place 's/\<meta name=\"description\" content=\".*\"\>/<meta name=\"description\" content=\"Learn the rules to the game of Copenhagen Hnefatafl\. Move your pieces until you achieve victory or lose\. Try not to get surrounded as the defenders and escape\.\"\>/' /var/www/html/rules.html
sed --in-place '/\<\\!\-\- Custom HTML head \-\-\>/a \        <link rel=\"canonical\" href=\"https:\/\/hnefatafl.org\" \/\>' /var/www/html/index.html

cat << EOF > /var/www/html/robots.txt
User-agent: *
Allow: /

Sitemap: https://hnefatafl.org/sitemap.xml
EOF


mkdir --parents /var/www/html/binaries/debian/
cp ../../hnefatafl-copenhagen_2.1.0-1_amd64.deb /var/www/html/binaries/debian/

mkdir --parents /var/www/html/binaries/nsis/
cp ../../hnefatafl-client-installer.exe /var/www/html/binaries/nsis/
cp ../../hnefatafl-client-installer-0.13.4.exe /var/www/html/binaries/nsis/
cp ../../hnefatafl-client-installer-1.0.0.exe /var/www/html/binaries/nsis/
cp ../../hnefatafl-client-installer-1.1.3.exe /var/www/html/binaries/nsis/
cp ../../hnefatafl-client-installer-1.1.4.exe /var/www/html/binaries/nsis/
cp ../../hnefatafl-client-installer-1.2.1.exe /var/www/html/binaries/nsis/
cp ../../hnefatafl-client-installer-2.0.3.exe /var/www/html/binaries/nsis/
cp ../../hnefatafl-client-installer-2.1.0.exe /var/www/html/binaries/nsis/

sscli -b https://hnefatafl.org -r /var/www/html/

mkdir /var/www/html/.well-known
echo "fb1c1fdb-d01d-4918-911f-f4cf4b0540a0" > /var/www/html/.well-known/org.flathub.VerifiedApps.txt

cp index-now/* /var/www/html/

# Install sscli with "npm i -g static-sitemap-cli".

# To update the 404 page:
#     Edit "/etc/apache2/apache.conf"
#     Add the line: "ErrorDocument 404 /404.html"
#     Restart Apache: systemctl restart apache2

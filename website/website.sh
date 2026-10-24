#! /bin/bash -ex

# PATH+=":$HOME/.cargo/bin"

mdbook build

rm -rf /var/www/html/css
rm -rf /var/www/html/fonts
rm -rf /var/www/html/images
rm -rf /var/www/html/screenshots

mv --force ./book/html/* /var/www/html/

./replace_strings.sh

cat << EOF > /var/www/html/robots.txt
User-agent: *
Allow: /

Sitemap: https://hnefatafl.org/sitemap.xml
EOF

mkdir --parents /var/www/html/binaries/nsis/
cp ../../www/hnefatafl-client-installer-*.exe /var/www/html/binaries/nsis/
cp -r ../../www/apt/ /var/www/html/
cp -r ../../www/homebrew/ /var/html/

mkdir /var/www/html/.well-known
echo "fb1c1fdb-d01d-4918-911f-f4cf4b0540a0" > /var/www/html/.well-known/org.flathub.VerifiedApps.txt

cp index-now/* /var/www/html/

# Install  "cargo install mdbook-sitemap".

# To update the 404 page:
#     Edit "/etc/apache2/apache.conf"
#     Add the line: "ErrorDocument 404 /404.html"
#     Restart Apache: systemctl restart apache2

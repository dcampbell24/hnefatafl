#! /bin/bash -ex

mdbook build --dest-dir /var/www/html/

cargo build --bin replace_strings --features website --no-default-features
../target/debug/replace_strings

cat << EOF > /var/www/html/robots.txt
User-agent: *
Allow: /

Sitemap: https://hnefatafl.org/sitemap.xml
EOF

mkdir --parents /var/www/html/binaries/nsis/
cp ../../www/hnefatafl-client-installer-*.exe /var/www/html/binaries/nsis/
cp -r ../../www/apt/ /var/www/html/

sscli -b https://hnefatafl.org -r /var/www/html/

mkdir /var/www/html/.well-known
echo "fb1c1fdb-d01d-4918-911f-f4cf4b0540a0" > /var/www/html/.well-known/org.flathub.VerifiedApps.txt

cp index-now/* /var/www/html/

# Install sscli with "npm i -g static-sitemap-cli".

# To update the 404 page:
#     Edit "/etc/apache2/apache.conf"
#     Add the line: "ErrorDocument 404 /404.html"
#     Restart Apache: systemctl restart apache2

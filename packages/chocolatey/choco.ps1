# Do this first!
# checksum -t sha256 -f .\tools\hnefatafl-client-installer-4.4.1.exe

cd hnefatafl-copenhagen/
choco pack
choco push --source https://push.chocolatey.org/

scp .\tools\hnefatafl-client-installer-4.4.1.exe root@hnefatafl.org:~/

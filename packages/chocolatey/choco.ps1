cd hnefatafl-copenhagen/
choco pack
choco push --source https://push.chocolatey.org/

scp .\tools\hnefatafl-client-installer-4.3.0.exe root@hnefatafl.org:~/
checksum -t sha256 -f .\tools\hnefatafl-client-installer-4.3.0.exe

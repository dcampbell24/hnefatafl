cd hnefatafl-copenhagen/
choco pack
choco push --source https://push.chocolatey.org/

scp .\tools\hnefatafl-client-installer-*.exe root@hnefatafl.org:~/www/

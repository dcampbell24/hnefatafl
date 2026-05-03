cd hnefatafl-copenhagen/

scp .\tools\hnefatafl-client-installer-*.exe root@hnefatafl.org:~/www/

choco pack
choco push --source https://push.chocolatey.org/

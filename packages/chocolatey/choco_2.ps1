cd hnefatafl-copenhagen/

scp .\tools\hnefatafl-client-installer-*.exe root@hnefatafl.org:~/www/
scp ..\..\..\target\release\hnefatafl-client.exe david@192.168.1.141:~

choco pack
choco push --source https://push.chocolatey.org/

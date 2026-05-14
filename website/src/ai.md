![AI](screenshots/ai.png)

## Artifcial Intelligence

I adapted TaflZero to my server. It is based off of AlphaGoZero by
Google. You can run it via `taflzero` on the Arch, Debian, and Fedora
installs, or via the source package. See `--help` for what options you can pass
it.

You'll have to create an account for it first on whatever server you will be
running it on. Run `hnefatafl-client` and log onto the server. Then, Create an
account for your AI, prefixing the username with `ai-taflzero`.

It can also be run as a service. Edit the file
`/etc/hnefatafl-ai-attacker.conf` or `/etc/hnefatafl-ai-defender.conf` and add

```sh
USERNAME=username
PASSWORD=password
```

Don't prefix the `USERNAME` with `ai-taflzero` here.

Then run

```sh
sudo systemctl start hnefatafl-ai-attacker
```

or

```sh
sudo systemctl start hnefatafl-ai-defender
```

If you want to change the settings for the AI further, you can edit
`/usr/lib/systemd/system/hnefatafl-ai-attacker.service` or
`/usr/lib/systemd/system/hnefatafl-ai-defender.service` and change the value of
`ExecStart`.

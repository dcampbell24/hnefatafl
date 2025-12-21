## Artifcial Intelligence

I am working on making a reasonably strong artificial intelligence, but it is
pretty weak at the moment. If you have the source package, You can run it via
`cargo run --release --bin hnefatafl-ai`. See `--help` for what options you can
pass it.

You'll have to create an account for it first on whatever server you will be
running it on. Run the client and log onto the server. Then, Create an account
for your AI, prefixing the username with `ai-`.

It can also be run as a service for the Arch and Debian installs. Edit the file
`/etc/hnefatafl-ai-attacker.conf` or `/etc/hnefatafl-ai-defender.conf` and add

```sh
USERNAME=username
PASSWORD=password
```

Don't prefix the `USERNAME` with `ai-` here.

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

By default this runs basic AI with a search depth of 4. The AI seems pretty
weak if you go below 4. You can increase the depth, but the AI may run very
slowly.

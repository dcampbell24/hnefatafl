cask "hnefatafl-copenhagen" do
  version "5.8.0"
  # Probably the output of a program named sha256 or
  # something similar with argument of v5.8.0-1.zip
  sha256 "fill me in"

  url "https://codeberg.org/dcampbell/hnefatafl/archive/v5.8.0-1.zip"
  name "hnefatafl-copenhagen"
  desc "Client that connects to a server"
  homepage "https://hnefatafl.org"
  manpage "hnefatafl-client.1.gz"

  # Or something similar, whatever you get as output from "just macos" in
  # the hnefatafl directory.
  app "hnefatafl-copenhagen.app"
end

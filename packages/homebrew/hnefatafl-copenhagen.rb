cask "hnefatafl-copenhagen" do
  name "hnefatafl-copenhagen"
  version "5.8.0"
  desc "Client that connects to a server"
  homepage "https://hnefatafl.org"
  url "https://hnefatafl.org/homebrew/hnefatafl-copenhagen.tar.gz"
  sha256 "05bfc820cdb8821a77728b5641699174b53a86796b5e9600e2954766bf9cced8"
  license "AGPL-3.0-or-later"

  test do
    system "true"
  end
end

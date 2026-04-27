cask "hnefatafl-copenhagen" do
  name "hnefatafl-copenhagen"
  version "5.8.0"
  desc "Client that connects to a server"
  homepage "https://hnefatafl.org"
  url "https://hnefatafl.org/homebrew/hnefatafl-copenhagen.tar.gz"
  sha256 "f191e0a9ce749bc028d21739fd87641046e5a9de37814acb0cb1208c89bd33d2"
  license "AGPL-3.0-or-later"

  test do
    system "true"
  end
end

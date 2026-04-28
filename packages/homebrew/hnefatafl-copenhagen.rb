cask "hnefatafl-copenhagen" do
  name "hnefatafl-copenhagen"
  version "5.8.0"
  desc "Client that connects to a server"
  homepage "https://hnefatafl.org"
  url "https://hnefatafl.org/homebrew/hnefatafl-copenhagen.tar.gz"
  sha256 "92f8775e3701410c8e09d55b0d62d23256109a74ac9c67466bdffdf1dbe8f1a8"
  license "AGPL-3.0-or-later"

  test do
    system "true"
  end
end

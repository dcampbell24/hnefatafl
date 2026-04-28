cask "hnefatafl-copenhagen" do
  name "hnefatafl-copenhagen"
  version "5.8.0"
  desc "Client that connects to a server"
  homepage "https://hnefatafl.org"
  url "https://hnefatafl.org/homebrew/hnefatafl-copenhagen.tar.gz"
  sha256 :no_check
  license "AGPL-3.0-or-later"
  depends_on macos: ">= :big_sur"

  test do
    system "true"
  end
end

cask "hnefatafl-copenhagen" do
  version "5.8.0"
  sha256 :no_check

  url "https://hnefatafl.org/homebrew/hnefatafl-copenhagen.tar.gz"
  name "Hnefatafl Copenhagen"
  desc "Client that connects to a server"
  homepage "https://hnefatafl.org/"

  depends_on macos: ">= :big_sur"

  app "hnefatafl-copenhagen.app"
end

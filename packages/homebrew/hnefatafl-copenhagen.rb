cask "hnefatafl-copenhagen" do
  version "6.2.0"
  sha256 :no_check

  url "https://hnefatafl.org/homebrew/hnefatafl-copenhagen-#{version}.tar.gz"
  name "Hnefatafl Copenhagen"
  desc "Copenhagen Hnefatafl client that connects to a server"
  homepage "https://hnefatafl.org/"

  depends_on macos: ">= :big_sur"

  app "hnefatafl-copenhagen.app"
end

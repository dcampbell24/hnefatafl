#! /bin/bash -e

INSERT="\
  <key>CFBundleIconFile</key>
  <string>shortcut.icns</string>
</dict>"

mkdir shortcut.iconset
sips -z 16 16     helmet_1024.png --out shortcut.iconset/icon_16x16.png
sips -z 32 32     helmet_1024.png --out shortcut.iconset/icon_16x16@2x.png
sips -z 32 32     helmet_1024.png --out shortcut.iconset/icon_32x32.png
sips -z 64 64     helmet_1024.png --out shortcut.iconset/icon_32x32@2x.png
sips -z 128 128   helmet_1024.png --out shortcut.iconset/icon_128x128.png
sips -z 256 256   helmet_1024.png --out shortcut.iconset/icon_128x128@2x.png
sips -z 256 256   helmet_1024.png --out shortcut.iconset/icon_256x256.png
sips -z 512 512   helmet_1024.png --out shortcut.iconset/icon_256x256@2x.png
sips -z 512 512   helmet_1024.png --out shortcut.iconset/icon_512x512.png
cp helmet_1024.png shortcut.iconset/icon_512x512@2x.png
iconutil -c icns shortcut.iconset
rm -R shortcut.iconset

cd ../..
cargo install cargo-bundle
cargo bundle --bin hnefatafl-client --features client --no-default-features --release

mkdir target/release/bundle/osx/hnefatafl-copenhagen.app/Contents/Resources
mv packages/homebrew/shortcut.icns target/release/bundle/osx/hnefatafl-copenhagen.app/Contents/Resources

cd target/release/bundle/osx/

cd hnefatafl-copenhagen.app/Contents
while IFS= read -r line; do
    if [[ $line == '</dict>' ]]; then
        echo "$INSERT"
    else
        echo "$line"
    fi
done < Info.plist > NewInfo.plist
mv NewInfo.plist Info.plist
cd ../..

tar -czvf hnefatafl-copenhagen.tar.gz hnefatafl-copenhagen.app
sha256sum hnefatafl-copenhagen.tar.gz
scp  hnefatafl-copenhagen.tar.gz david@192.168.1.141:~

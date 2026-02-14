#! /bin/bash -e

PACKAGE='hnefatafl-copenhagen_5.1.0-2_amd64.deb'

packages/debian/deb.sh

mkdir --parents packages/debian/apt/pool/main
mkdir --parents packages/debian/apt/dists/stable/main/binary-amd64

cp target/debian/${PACKAGE} packages/debian/apt/pool/main

cd packages/debian/apt
dpkg-scanpackages --arch amd64 pool/ > dists/stable/main/binary-amd64/Packages
cat dists/stable/main/binary-amd64/Packages | lzma --keep > dists/stable/main/binary-amd64/Packages.xz

cd dists/stable/

cat > Release << EOF
Origin: Hnefatafl Org
Label: Hnefatafl Copenhagen
Suite: stable
Codename: stable
Version: 5.1.0-2
Architectures: amd64
Components: main
Description: A software repository containing Hnefatafl Copenhagen. Discord: https://discord.gg/h56CAHEBXd
Date: $(date -Ru)
EOF

do_hash() {
    HASH_NAME=$1
    HASH_CMD=$2
    echo "${HASH_NAME}:"
    for f in $(find -type f); do
        f=$(echo $f | cut -c3-) # remove ./ prefix
        if [ "$f" = 'Release' ]; then
            continue
        fi
        echo " $(${HASH_CMD} ${f}  | cut -d" " -f1) $(wc -c $f)"
    done
}

do_hash 'MD5Sum' 'md5sum' >> 'Release'
do_hash 'SHA1' 'sha1sum' >> 'Release'
do_hash 'SHA256' 'sha256sum' >> 'Release'

cat Release | gpg -abs > Release.gpg
cat Release | gpg -abs --clearsign > InRelease

scp -r ../../../apt/ root@hnefatafl.org:~/www/


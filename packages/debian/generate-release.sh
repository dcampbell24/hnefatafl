#! /bin/bash -e

PACKAGE='hnefatafl-copenhagen_4.4.1-1_amd64.deb'

mkdir --parents apt/pool/main
mkdir --parents apt/dists/stable/main/binary-amd64

cp ../../target/debian/${PACKAGE} apt/pool/main

cd apt
dpkg-scanpackages --arch amd64 pool/ > dists/stable/main/binary-amd64/Packages
cat dists/stable/main/binary-amd64/Packages | lzma --keep > dists/stable/main/binary-amd64/Packages.xz

cd dists/stable/

cat > Release << EOF
Origin: Hnefatafl Org
Label: Hnefatafl Copenhagen
Suite: stable
Codename: stable
Version: 4.3.0-1
Architectures: amd64
Components: main
Description: A software repository containing Hnefatafl Copenhagen
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

# /etc/apt/sources.list.d/hnefatafl.list
# deb [arch=amd64 signed-by=/etc/apt/keyrings/packages.hnefatafl.org.asc] http://127.0.0.1:8000/apt stable main

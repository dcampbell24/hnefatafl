#! /bin/bash -e

PACKAGE='hnefatafl-copenhagen_4.2.2-1_amd64.deb'

mkdir --parents apt-repo/pool/main
mkdir --parents apt-repo/dists/stable

cp ../../target/debian/${PACKAGE} apt-repo/pool/main

cd apt-repo
dpkg-scanpackages --arch amd64 pool/ > dists/stable/main/binary-amd64/Packages
cat dists/stable/main/binary-amd64/Packages | lzma --keep > dists/stable/main/binary-amd64/Packages.xz

cd dists/stable/

cat > Release << EOF
Origin: Hnefatafl Org
Label: Hnefatafl Copenhagen
Suite: stable
Codename: stable
Version: 4.2.2-1
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


# echo "deb [arch=amd64] http://127.0.0.1:8000/apt-repo stable main" >  /etc/apt/sources.list.d/hnefatafl.list
#!/bin/bash

RUST_MIN_STACK=268435456 rpmbuild -ba hnefatafl-copenhagen.spec
rpmlint --verbose hnefatafl-copenhagen.spec ../RPMS/x86_64/hnefatafl-copenhagen-5.8.0-1.fc44.x86_64.rpm 

# copr-cli build hnefatafl-copenhagen ~/rpmbuild/SRPMS/hnefatafl-copenhagen-5.6.1-1.fc43.src.rpm
# sudo dnf install google-noto-fonts-all.noarch R-fontBitstreamVera.noarch
# sudo dnf upgrade --refresh

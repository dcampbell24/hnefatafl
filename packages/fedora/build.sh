#!/bin/bash

RUST_MIN_STACK=134217728 rpmbuild -ba hnefatafl-copenhagen.spec
rpmlint --verbose ~/rpmbuild/RPMS/x86_64/hnefatafl-copenhagen-5.6.1-2.fc43.x86_64.rpm

# copr-cli build hnefatafl-copenhagen ~/rpmbuild/SRPMS/hnefatafl-copenhagen-5.6.1-1.fc43.src.rpm
# sudo dnf install google-noto-fonts-all.noarch R-fontBitstreamVera.noarch

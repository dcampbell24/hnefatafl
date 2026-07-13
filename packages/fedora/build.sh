#!/bin/bash

rpmbuild -ba hnefatafl-copenhagen.spec
rpmlint --verbose hnefatafl-copenhagen.spec ../RPMS/x86_64/hnefatafl-copenhagen-6.1.1-1.fc44.x86_64.rpm

# copr-cli build hnefatafl-copenhagen ~/rpmbuild/SRPMS/hnefatafl-copenhagen-6.1.1-1.fc44.src.rpm
# sudo dnf upgrade --refresh

#!/bin/bash

rpmbuild -ba hnefatafl-copenhagen.spec
rpmlint --verbose hnefatafl-copenhagen.spec ../RPMS/x86_64/hnefatafl-copenhagen-7.0.0-1.fc45.x86_64.rpm
copr-cli build hnefatafl-copenhagen ~/rpmbuild/SRPMS/hnefatafl-copenhagen-7.0.0-1.fc45.src.rpm

# sudo dnf upgrade --refresh

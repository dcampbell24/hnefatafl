#!/bin/bash

RUST_MIN_STACK=67108864 rpmbuild -ba hnefatafl-copenhagen.spec
rpmlint --verbose ~/rpmbuild/RPMS/x86_64/hnefatafl-copenhagen-5.6.1-1.fc43.x86_64.rpm

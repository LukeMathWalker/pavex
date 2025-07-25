#!/bin/bash

set -o pipefail

rm -rf starter
PAVEXC_TEMPLATE_VERSION_REQ="0.2" PAVEX_PAVEXC=pavexc pavex new starter
rm -rf starter/.git

#!/bin/bash

set -x

cd /root/setup
npx ts-node setup /root/programs/nezha_staking.so /root/programs/nezha_vrf_mock.so

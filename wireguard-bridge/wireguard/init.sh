#!/bin/sh
wg-quick up wg0 || exit 1
if test -n "$TESTNET2"; then
	service="lottery-testnet2";
else
	service="lottery";
fi;
cat >Caddyfile <<EOF
:80

reverse_proxy $service.internal.service {
	header_up Host {upstream_hostport}
}
EOF
caddy run

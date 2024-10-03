from alpine:latest
RUN	apk update && apk upgrade && apk add wireguard-tools caddy
COPY init.sh /init.sh
ENTRYPOINT [ "/init.sh" ]

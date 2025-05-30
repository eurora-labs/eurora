FROM envoyproxy/envoy:v1.17-latest

EXPOSE 8080
ENV ENVOY_UID=0

WORKDIR /etc/envoy

COPY config/envoy/envoy.yaml /etc/envoy/envoy.yaml
CMD /usr/local/bin/envoy -c /etc/envoy/envoy.yaml
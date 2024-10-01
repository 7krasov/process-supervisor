FROM php:8.3-cli
RUN apt update && apt install procps net-tools -y
RUN mkdir -p /var/app
COPY ../../target/debug/processes-supervisor /var/app
WORKDIR /var/app
CMD [ "/var/app/processes-supervisor" ]

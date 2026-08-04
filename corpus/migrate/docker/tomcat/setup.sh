#!/bin/sh
set -eu
mkdir -p /var/lib/tomcat/conf /var/lib/tomcat/webapps /var/lib/tomcat/work
if [ ! -e /var/lib/tomcat/conf/server.xml ]; then
  cp -R --no-preserve=mode,ownership /tomcat/conf/. /var/lib/tomcat/conf/
fi
mkdir -p /var/lib/tomcat/conf/Catalina/localhost
sed -i 's/port="8005" shutdown="SHUTDOWN"/port="-1" shutdown="SHUTDOWN"/' /var/lib/tomcat/conf/server.xml
ln -sfn /var/log/tomcat /var/lib/tomcat/logs

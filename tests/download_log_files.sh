if [ ! -d "/tmp/testlogs" ]; then
    echo "log files not found, downloading..."
    mkdir /tmp/testlogs
    wget http://autotest.ardupilot.org/ArduSub-test.tlog --directory-prefix=/tmp/testlogs/
fi

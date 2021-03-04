if [ ! -d "/tmp/testlogs" ]; then
    echo "log files not found, downloading..."
    mkdir /tmp/testlogs
    wget https://autotest.ardupilot.org/history/2021-03-03-23:03/ArduSub-test.tlog --directory-prefix=/tmp/testlogs/
fi

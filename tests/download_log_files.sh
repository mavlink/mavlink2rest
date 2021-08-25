if [ ! -d "/tmp/testlogs" ]; then
    echo "log files not found, downloading..."
    mkdir /tmp/testlogs
    wget https://autotest.ardupilot.org/history/2021-08-24-22:08/ArduSub-test.tlog --directory-prefix=/tmp/testlogs/
fi

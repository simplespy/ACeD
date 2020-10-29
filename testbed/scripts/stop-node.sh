#!/bin/bash

echo "Killing Scale processes"

pkill -9 system_rust
wait $!
# pids=`cat /home/ubuntu/log/prism.pid`
#
# kill_pids=""
#
# for pid in $pids; do
#	echo "Killing $pid"
#	kill $pid
#	kill_pids="$kill_pids $!"
# done
#
# echo "Waiting for processes to exit"
# for pid in $kill_pids; do
#	wait $pid
# done
#
#echo "All process exited"

rm -f /root/log/scale.pid

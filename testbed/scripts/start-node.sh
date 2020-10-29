#!/bin/bash

function wait_for_line() {
	tail -F -n1000 $1 | grep -q "$2"
}

echo "Launch scalechain node"

for script in /root/payload/scale-payload/*.sh ; do
	[ -f $script ] || continue
	node_name=$(basename $script .sh)
	echo "Launch $node_name"
	export RUST_LOG=info
	nohup bash $script &> /root/log/$node_name.log &
	echo "$!" >> /root/log/scale.pid
done

echo "Waiting for API server to start"
for script in /root/payload/scale-payload/*.sh; do
	[ -f "$script" ] || continue
	node_name=`basename $script .sh`
	wait_for_line /root/log/$node_name.log 'API server listening'
	echo "Node $node_name started"
done




#!/bin/bash
trap kill_test INT

bin="../target/debug/system_rust"
contract_config_file='./contract-network.json'
contract_address=$(jq -r ".contract_address" $contract_config_file)
rpc_url=$(jq -r ".rpc_url" $contract_config_file)

if [ "$#" -lt 1 ]; then
	echo "Usgae ./run_experiment.sh help"
	exit 0
fi



function kill_test() {
	stop_time=$(date +%s)
	echo "STOP ${stop_time}" >> experiment.txt
	for pid in $pids; do 
		echo "Kill $pid"
		kill $pid
	done	
}

# $1 id, $2 total, $3 side nodes, $4 scale_id, $5 num_scale, $6 num_account
# $7 start_time
function start_node() {
	p2p_port=$(expr 40000 + $1)
	api_port=$(expr 41000 + $1)
	peer_addr="127.0.0.1:"${p2p_port}
	api_addr="127.0.0.1:"${api_port}
	keyfile='keyfile/node'$1
	account_num=$( expr $1 % $6 + 1 )
	account="--account accounts/account"$account_num

	known_peers=""	
	for(( a=1 ; a<=$2 ; a++ )); do
		if [ $a -ne $1 ] ; then
			port=$(expr 40000 + $a)
			addr=127.0.0.1:$port
			known_peers="$known_peers -c $addr"
		fi
	done
	
	echo "RUST_LOG=info $bin -i $peer_addr ${known_peers} $3 --api_addr $api_addr $account --key $keyfile --scale_id $4 -n $5 -l ../src/LDPC_codes -j "./scripts/abi.json" -b "../go-bls" --contract_address ${contract_address} --rpc_url ${rpc_url} --start_time ${start_time}"
	RUST_LOG=info $bin -i $peer_addr ${known_peers} $3 --api_addr "$api_addr" $account -t $7 --key "$keyfile" --scale_id $4 -n $5 -l "../src/LDPC_codes" -j "./scripts/abi.json" -b "../go-bls" -f ${contract_address} -u ${rpc_url} --start_time ${start_time}&
	pid="$!"
	pids="$pids $pid"
}


function start_trans {
	sh ./scripts/start.sh
}

function start_ping {
	for (( i = 1; i<=$1 ; i++ )); do
		api_port=$(expr 41000 + $i)
		curl "localhost:${api_port}/server/ping" 
	done
	
}

function config() {
	start_file="scripts/start.sh"
	side_file="side_node"
	#tel_file="telematics/nodes.txt"
	total=$(expr $1 + $2)

	# neighbors
	for (( i = 1; i<=$total ; i++ )); do
		node=$(expr 40000 + $i)
		#echo "127.0.0.1:${node}">> $neighbor_file
	done

	# sidenodes + start
	rm $side_file
	#rm $tel_file
	rm $start_file
	echo "#!/bin/bash" >> $start_file
	chmod +x $start_file
	for (( i = $1+1 ; i<=$total ; i++)); do
		node=$(expr 40000 + $i)
		echo "127.0.0.1:${node}" >> $side_file
		api=$(expr 41000 + $i)
		echo "curl "localhost:${api}/transaction-generator/start?interval=60000"" >> $start_file
		#echo "$i,127.0.0.1,$api" >> $tel_file
	done	
}

# $1 number scale $2 number sides $3 num_account
function start_local() {
	if [ "$#" -ne 4 ]; then
		echo "Usgae ./run_experiment.sh start_local <NUM SCALE NODE> <NUM SIDE NODE> <NUM ETH ACCOUNT> <SLOT TIME>"
		exit 0
	fi

	# config files
	config $1 $2
	pids="" 

	# reset chain
	echo "$bin resetChain --contract_addr ${contract_address} --node_url ${rpc_url}"
	$bin resetChain --contract_addr ${contract_address} --node_url ${rpc_url}
	if [ $? -ne 0 ]; then
		echo "Fail: $bin resetChain"
		exit 0
	fi

	total=$(expr $1 + $2)

	side_nodes=""
	for(( a=$1+1 ; a<=$total ; a++ )); do
		port=$(expr 40000 + $a)
		addr=127.0.0.1:$port
		side_nodes="$side_nodes -r $addr"
	done
	rm "nodes.txt"
	start_time=$(gdate +%s.%N)
	echo $start_time
	for (( i=1 ; i<=$total ; i++ )); do 
		scale_id=$i
		if [ $i -gt $1 ] ; then
			scale_id=0
		fi
		start_node $i $total "$side_nodes" ${scale_id} $1 $3 $4 ${start_time}

		# nodes.txt
		port=$(expr 40000 + $i)
		api_port=$(expr 41000 + $i)
		echo "node_${i},$i,127.0.0.1,$port,${api_port},${scale_id}" >> "nodes.txt"
	done
	start_time=$(date +%s)
	rm -f experiment.txt
	echo "START ${start_time}" >> experiment.txt
	sleep 1
	sh ./scripts/start.sh

	echo "pids $pids"

	for pid in $pids; do 
		wait $pid
	done
}

function get_curr_state
{
	$bin getCurrState --contract_addr $contract_address --node_url ${rpc_url}
}

function reset_chain 
{
	$bin resetChain --contract_addr $contract_address --node_url ${rpc_url}
}

function get_scale_nodes
{
	$bin getScaleNodes --contract_addr $contract_address --node_url ${rpc_url}
}

function add_scale_nodes 
{
	if [ "$#" -ne 1 ]; then
		echo "Usage add-scale-nodes <NUM>"
		exit 0
	fi
	contract_master="accounts/account0"
	for (( i=6; i<=$1 ; i++ )); do
		new_account="accounts/account$i"
		echo $new_address
		keyfile="keyfile/node$i"
		ip="127.0.0.1"
		$bin addScaleNode --contract_addr ${contract_address} --node_url ${rpc_url} --account ${contract_master} --keyfile ${keyfile} --ip_addr ${ip} --new_account ${new_account}
	done
	echo "Curr scale nodes"
	get_scale_nodes
}

case $1 in 
	help) 
		cat <<- EOF
		Helper funciton 

			Run local experiment 
				start num_scale num_side num_account slot_time
				gen		
				get-curr-state
				reset-chain
				get-scale-nodes
				add-scale-nodes num_node
		EOF
		;;	
	start)
		start_local $2 $3 $4 $5 ;;
	gen)
		start_trans ;;
	ping)
		start_ping $2 ;;
	get-curr-state)
		get_curr_state ;;
	reset-chain)
		reset_chain ;;
	get-scale-nodes)
		get_scale_nodes ;;
	add-scale-nodes)
		add_scale_nodes $2 ;;
	*) 
		echo "unknown command" ;;
esac
		

#!/bin/bash

trap close_instances INT

builder="bench"
region=4 # seattle

# Seattle Sill Amsterdam Tokyo Chicago Paris Atlanta  Dallas Singapore New-Jersey   4 12 9 25 2 24 6 3 40 1
declare -a regions=(4 12 9 25 2 24 6 3 40 1)
regions_len=${#regions[@]}
bin="../target/debug/system_rust"

plan=205 # 10 dollar per month =  1cpu 2G ram
os=270   # ubuntu os
script_id=736557 # startup script id

s3_prefix="https://ewr1.vultrobjects.com/scale-payload/payload"

if [ "$#" -lt 1 ]; then
	echo "Usgae ./run_experiment.sh help"
	exit 0
fi

function close_instances
{
	stop_experiment
}

function build {
	echo "copy src files"
	ssh ${builder} -- 'mkdir ~/scale'
	rsync -ar ../src/ ${builder}:~/scale/src
	rsync -ar ../coded_merkle_tree/ ${builder}:~/scale/coded_merkle_tree
	rsync -ar ../Cargo.toml ${builder}:~/scale/Cargo.toml
	echo "build source"
	ssh ${builder} -- 'cd ~/scale && ~/.cargo/bin/cargo build --release' &> log/build.log
	echo "strip symbol"
	ssh ${builder} -- 'cp ~/scale/target/release/system_rust ~/scale/target/release/system_rust-copy && strip ~/scale/target/release/system_rust-copy'
	ssh ${builder} -- 'mv ~/scale/target/release/system_rust ~/scale/target/release/system_rust1'
	tput setaf 2
	echo "Finished"
	tput sgr0
}

function fix_instances() {
	instances=$(vultr-cli server list)
	for instance in $instances; 
	do
		echo $instance
	done
}

function start_instances() {
	if [ $# -ne 1 ]; then 
		tput setaf 1 
		echo "Required number of instances"
		tput sgr 0
	 	exit 1
	fi
	echo "Really"
	select yn in "Yes" "No" ; do
		case $yn in
			Yes ) break ;;
			No ) echo "Nothing happened."; exit ;;
			* ) echo "Unrecognized. $yn"; exit ;;
		esac
	done
	echo "Launching $1 instances"
  local instances=""

	for (( i=0 ; i<$1 ; i++ )); do
		j=$(expr $i % $regions_len)
		region=${regions[$j]}
		instances="$instances $(vultr-cli server create --region $region --plan $plan --os $os --script-id ${script_id} --notify true | grep -Eo '[0-9]+$' )"
	done

	sleep 10
	#tput rc
	#tput el
	echo "Instances launched"
	rm -f instances.txt
	rm -f ~/.ssh/config.d/exp

	local details=""
	for id in $instances;
	do
		echo $id
		while [ 1 ]
		do
			ip=$( vultr-cli server info $id | grep "Main IP" | awk '{print $3}' )
			if echo $ip | grep '0.0.0.0' ; then
				echo 'wait for ip be assigned'
				sleep 2
				continue
			else
				echo "$id,$ip" >> instances.txt
				echo "Host $id" >> ~/.ssh/config.d/exp
				echo "		Hostname $ip" >> ~/.ssh/config.d/exp
				echo "		User root" >> ~/.ssh/config.d/exp
				echo "		IdentityFile ~/.ssh/id_rsa" >> ~/.ssh/config.d/exp
				echo "    StrictHostKeyChecking no" >> ~/.ssh/config.d/exp
				echo "    UserKnownHostsFile=/dev/null" >> ~/.ssh/config.d/exp
				echo "" >> ~/.ssh/config.d/exp
				break
			fi
		done
	done
	tput setaf 2
	echo "Instances started"
	tput sgr0
}	

function read_instances 
{
	#vultr-cli server list | tail -2 | awk '{ printf "%10s\n", $1}' 
	while [ "$remaining" -gt "0" ]
	do 
		#tput rc
		#tput el
		instances="$instances $(vultr-cli server create --region $region --plan $plan --os $os --script-id ${script_id} --notify true | grep -Eo '[0-9]+$' )"
		remaining=$(expr $remaining - 1 )	
	done
}

function start_ping_single 
{
	curl "http://$3:$4/server/ping"
}

function query_side_api
{
	local nodes=$(cat nodes.txt)
	echo $nodes
	local pids=''
	for node in $nodes; do
		local name
		local host
		local pubip
		local apiport
		local scale_id
		IFS=',' read -r name host pubip _ api_port scale_id <<< "$node"

		if [ ${scale_id} -eq 0 ]; then
			echo "$name $host $pubip $apiport" 
			$1_single $name $host $pubip $api_port &
			pids="$pids $!"
		fi
	done
	for pid in $pids; do
		wait $pid
	done
}

function query_api 
{
	local nodes=$(cat nodes.txt)
	echo $nodes
	local pids=''
	for node in $nodes; do
		local name
		local host
		local pubip
		local apiport
		local scale_id

		IFS=',' read -r name host pubip _ api_port scale_id <<< "$node"
		$1_single $name $host $pubip $api_port &
		pids="$pids $!"
	done
	for pid in $pids; do
		wait $pid
	done
}

function stop_instances
{
	echo "Really?"
	select yn in "Yes" "No"; do
		case $yn in
			Yes ) break ;;
			No ) echo "Nothing happened."; exit ;;
		esac
	done

	local instances=$(cat instances.txt)
	for instance in $instances ;
	do
		local id
		local ip
		IFS=',' read -r id ip <<< "$instance"
		vultr-cli server delete $id	> log/vultr_stop.log
	done

	tput setaf 2
	echo "Instances terminated"
	tput sgr0
}

function execute_on_all
{
	# $1: execute function '$1_single'
	# ${@:2}: extra params of the function
	local instances=`cat instances.txt`
	local pids=""
	for instance in $instances ;
	do
		local id
		local ip
		IFS=',' read -r id ip <<< "$instance"
		#tput rc
		#tput el
		echo -n "Executing $1 on $id"
		$1_single $id ${@:2} &>log/${id}_${1}.log &
		pids="$pids $!"
	done
	for pid in $pids ;
	do
		#tput rc
		#tput el
		#echo -n "Waiting for job $pid to finish"
		if ! wait $pid; then
			tput rc
			tput el
			tput setaf 1
			echo "Task $pid failed"
			tput sgr0
			tput sc
		fi
	done
	#tput rc
	#tput el
	tput setaf 2
	echo "Finished"
	tput sgr0
}

function sync_payload_direct
{
	execute_on_all sync_payload_direct
}

function sync_payload_direct_single
{
	echo "Uploading nodes $1"
	ssh $1 -- "rm -rf /root/payload && rm -rf /root/log && mkdir -p /root/payload"
	scp payload/common.tar.gz $1:/root/payload/
	scp payload/$1.tar.gz $1:/root/payload/
	ssh $1 -- "tar xf /root/payload/$1.tar.gz -C /root/payload && tar xf /root/payload/common.tar.gz -C /root/payload"
}

function sync_payload
{
	if [ $1 -eq 1 ]; then
		echo "Sync payload to s3"
		s3cmd --quiet rm --recursive s3://scale-payload/payload
		s3cmd sync -P payload s3://scale-payload
	fi
	echo "Instances get payload"
	execute_on_all sync_payload
}

function sync_time
{
	execute_on_all sync_time
}

function sync_time_single
{
	ssh $1 -- "apt -y install ntp"
}

function sync_payload_single
{
	ssh $1 -- "rm -rf /root/payload && rm -f /root/*.tar.gz && wget ${s3_prefix}/$1.tar.gz -O /root/local.tar.gz && wget ${s3_prefix}/common.tar.gz -O /root/common.tar.gz && mkdir -p /root/payload && tar xf /root/local.tar.gz -C /root/payload/ && tar xf /root/common.tar.gz -C /root/payload/ && ntpq -pn"
}



function start_scale
{
	execute_on_all start_scale
	start_time=`date +%s`
}

function start_trans
{
	query_side_api start_trans
}

function start_trans_single
{
	echo "$3 $4"
	curl "http://$3:$4/transaction-generator/start?interval=60000"
}

function start_scale_single
{
	ssh $1 -- 'mkdir -p /root/log && bash /root/payload/scripts/start-node.sh &>/root/log/start.log'
}

function stop_experiment
{
	execute_on_all stop_scale
	echo "Stopped Scale nodes"
	echo "STOP $start_time" >> experiment.txt
}

function stop_scale_single
{
	ssh $1 -- 'bash /root/payload/scripts/stop-node.sh &>/root/log/stop.log'
}

function run_experiment
{
	echo "Reset chain"
	bash ./local_exp.sh reset-chain
	echo "Starting Scale nodes"

	start_scale
	echo "All nodes started, starting transaction generation"
	if [ $# -eq 1 ]; then
		echo "Transaction generation type is set to $1"
		local txtype=$1
		set_tx_type $txtype
	fi
	start_trans
	rm -f experiment.txt
	echo "START $start_time" >> experiment.txt
	echo "Running experiment"
}

function prepare_payload
{
	# $1: topology file to use
	if [ $# -ne 4 ]; then
		tput setaf 1
		echo "Required: topology file, number scale nodes, slot time and contract config"
		tput sgr0
		exit 1
	fi
	echo "Deleting existing files"
	rm -rf payload
	mkdir -p payload
	mkdir -p payload/common/binary
	mkdir -p payload/common/scripts

	echo "Download binaries"
	scp bench:~/scale/target/release/system_rust-copy payload/common/binary/system_rust
	cp scripts/start-node.sh payload/common/scripts/start-node.sh
	cp scripts/stop-node.sh payload/common/scripts/stop-node.sh
	cp scripts/abi.json payload/common/binary/abi.json
	scp bench:~/scale/go-bls/sign payload/common/binary/sign
	scp bench:~/scale/go-bls/aggregate payload/common/binary/aggregate
	scp bench:~/.cargo/bin/ethabi payload/common/binary/ethabi
	cp -r LDPC_codes  payload/common/LDPC_codes

	echo "Generate scale config files and keypairs for each node"
	local start_time=$(gdate +%s.%N)
	python3 scripts/gen_payload.py instances.txt $1 $2 $2 $3 $4 ${start_time}

	echo "Compressing payload files"
	local instances=`cat instances.txt`
	local instance_ids=""
	for instance in $instances ;
	do
		local id
		IFS=',' read -r id _ <<< "$instance"
		tar cvzf payload/$id.tar.gz -C payload/$id . &> /dev/null
		rm -rf payload/$id
	done
	tar cvzf payload/common.tar.gz -C payload/common . &> /dev/null
	rm -rf payload/common

	tput setaf 2
	echo "Payload written"
	tput sgr0
}

function read_log 
{
	nodes=$(cat nodes.txt)	
	echo $1
	for node in $nodes; do
		local name
		local id
		local ip
		IFS=',' read -r name id ip _ _ _ <<< "$node"
		if [ "$name" == "$1" ] ; then
			echo "name $name $1 "
			ssh $id -- "cat /root/log/$name.log | less"
		fi
	done
}

function config_contract 
{
	# call binary to configure contract
	if [ ! -f "$bin" ]; then
		echo "Run cargo build"
	fi	
	
	nodes=$(cat nodes.txt)
	for node in $nodes; do
		local name
		local id
		local ip
		local scale_id
		IFS=',' read -r name id ip _ _ scale_id <<< "$node"
		if [ ${scale_id} -ne 0 ]; then
			echo "$name ${scale_id}"
			account="accounts/account${scale_id}"
			key="keyfile/node${scale_id}"
			${bin} addScaleNode --account ${account} --keyfile ${key} --ip_addr ${ip}
		fi
	done

	$bin resetChain
}

# $1 is contract config file
function reset_chain
{
	local contract_address=$(jq -r ".contract_address" $1)
	local rpc_url=$(jq -r ".rpc_url" $1)
	$bin resetChain --contract_addr ${contract_address} --node_url ${rpc_url}
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
	for (( i=1; i<=$1 ; i++ )); do
		new_account="accounts/account$i"
		echo $new_address
		keyfile="keyfile/node$i"
		ip="127.0.0.1"
		$bin addScaleNode --contract_addr ${contract_address} --node_url ${rpc_url} --account ${contract_master} --keyfile ${keyfile} --ip_addr ${ip} --new_account ${new_account}
	done
	echo "Curr scale nodes"
	get_scale_nodes
}

function show
{
	mkdir logData
	./telematics/telematics log 1 3600 nodes.txt ./telematics/rrDdata	
}

# $1 which node $2 content 
function plot
{
	start_time=$(grep 'START' experiment.txt | awk '{print $2}')
	stop_time=$(grep 'STOP' experiment.txt | awk '{print $2}')
	duration=$(expr $stop_time -  $start_time)
	rm output.png
	./telematics/telematics plot -nodelist nodes.txt -node $1 -content $2 -dataDir rrdData -start ${start_time} -duration $duration
	open output.png
}

function analyze
{
	outdir="data/$1"
	mkdir -p $outdir
	cp nodes.txt $outdir
	mv logData $outdir
	cd ./analysis 
	python plot.py ../${outdir}/logData ../$outdir
	cd ../
}

case $1 in 
	help) 
		cat <<- EOF
		Helper funciton 

			Run local experiment 
				build
				start-instances num_node
				stop-instances
				prepare-payload topology num_scale_nodes slot_time 
				sync_payload
				run-exp
				stop-exp
				start-ping
				start-trans
				read-log name
				config_contract

		Workflow:
			build
			start-instances
		  prepare-payload
			sync-payload
			reset-chain
			run-exp
			show
			stop-exp
			stop-instances	
			analyze output_name
			
		EOF
		;;
	build)
		build ;;	
	read-instances)
		read_instances ;;
	start-instances)
		start_instances $2 ;;
	stop-instances)
		stop_instances ;;
	prepare-payload)
		prepare_payload $2 $3 $4 $5 ;;
	sync-payload)
		sync_payload $2 ;;
	sync-payload-direct)
		sync_payload_direct ;;
	start-trans)
		start_trans ;;
	run-exp)
		run_experiment ;;
	stop-exp)
		stop_experiment ;;
	start-ping)
		query_api start_ping ;;
	read-log)
		read_log $2 ;;
	show)
		show ;;
	plot)
		plot $2 $3 ;;
	config-contract)
		config_contract ;;
	reset-chain)
		reset_chain $2 ;;
	get-scale-nodes)
		get_scale_nodes ;;
	add-scale-nodes)j
		add_scale_nodes $2 ;;
	fix)
		fix_instances ;;
	sync-time) 
		sync_time ;;
	analyze)
		analyze $2 ;;
	*)
		tput setaf 1
		echo "Unrecognized subcommand $1"
		tput sgr0 ;;
esac
		

import sys
import os
import json
import subprocess
from shutil import copyfile

template = """
cd /root/payload/binary && RUST_LOG=info /root/payload/binary/system_rust -i {ip}:{p2p_port} {known_peers} {side_nodes} --api_addr {ip}:{api_port} --account /root/payload/scale-payload/account{account_name} --key /root/payload/scale-payload/node{node_name} --scale_id {scale_id} -l /root/payload/LDPC_codes -n {num_scale} -t {slot_time} -b /root/payload/binary -j /root/payload/binary/abi.json --contract_addr {contract_address} --node_url {rpc_url} --start_time {start_time}
"""

instances_file = sys.argv[1]
instances = []
next_free_port = []

topology_file = sys.argv[2]
topo = {}

num_scale = int(sys.argv[3])

num_account = 10 #int(sys.argv[4])
print("num_account", num_account)

slot_time = float(sys.argv[5])

contract_config_file = sys.argv[6]
contract_config = {}

start_time = sys.argv[7]

# load instances
with open(instances_file) as f:
    for line in f:
        i = line.rstrip().split(",")
        instances.append(i)
        next_free_port.append(6000)

# load nodes
with open(topology_file) as f:
    topo = json.load(f)

with open(contract_config_file) as f:
    contract_config = json.load(f)

instance_idx = 0
instances_tot = len(instances)

nodes = {}

# assign ports and hosts for each node
for node in topo['nodes']:
    this = {}
    this['host'] = instances[instance_idx][0]
    this['pubfacing_ip'] = instances[instance_idx][1]
    this['p2p_port'] = next_free_port[instance_idx]
    next_free_port[instance_idx] += 1
    this['api_port'] = next_free_port[instance_idx]
    next_free_port[instance_idx] += 1
    # scale node id
    node_idx = int(node[5:]) 
    scale_id = int(node_idx)
    if scale_id > num_scale:
        scale_id = 0
    else:
        scale_id = node_idx
    this['scale_id'] = scale_id 
    nodes[node] = this
    # use the next instance
    instance_idx += 1
    if instance_idx == instances_tot:
        instance_idx = 0

side_nodes = [] 
for name, node in nodes.items():
    if node['scale_id'] == 0:
        side_nodes.append('-r {}:{}'.format(node['pubfacing_ip'], node['p2p_port']))
side_nodes = ' '.join(side_nodes)

# generate startup script for each node
for name, node in nodes.items():
    peers = []
    for c in topo['connections']:
        if c['from'] == name:
            dst = c['to']
            peers.append('-c {}:{}'.format(nodes[dst]['pubfacing_ip'], nodes[dst]['p2p_port']))
    known_peers = ' '.join(peers)
    node_idx = int(name[5:])
    
    account_name = (node_idx % num_account)
    if account_name == 0:
        account_name = num_account

    startup_str = template.format(
            node_name=node_idx, ip=node['pubfacing_ip'], api_port=node['api_port'], 
            p2p_port=node['p2p_port'], known_peers=known_peers, side_nodes=side_nodes, 
            scale_id=node['scale_id'], num_scale=num_scale, account_name=account_name,
            slot_time=slot_time, contract_address=contract_config['contract_address'],
            rpc_url=contract_config['rpc_url'], 
            start_time=start_time
            ).strip()

    os.makedirs("payload/{}/scale-payload".format(node['host']), exist_ok=True)
    with open("payload/{}/scale-payload/{}.sh".format(node['host'], name), "w") as f:
        f.write(startup_str)

# prepare keyfile and account info for each node
for name, node in nodes.items():
    node_idx = int(name[5:]) 
    account_name = (node_idx % num_account)
    if account_name == 0:
        account_name = num_account

    account_file = "accounts/account" + str(account_name);
    keyfile = "keyfile/node" + str(node_idx)
    copyfile(account_file, "payload/{}/scale-payload/{}".format(node['host'], "account"+str(account_name)))
    copyfile(keyfile, "payload/{}/scale-payload/{}".format(node['host'], "node"+str(node_idx)))

# write out node-host association
with open("nodes.txt", 'w') as f:
    for name, node in nodes.items():
        f.write("{},{},{},{},{},{}\n".format(name, node['host'], node['pubfacing_ip'], node['p2p_port'], node['api_port'], node['scale_id']))


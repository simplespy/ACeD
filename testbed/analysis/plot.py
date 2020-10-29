#!/usr/bin/env python

import matplotlib.pyplot as plt
import sys
import statistics
import os

def avg_nodes(i, data):
    s = 0.0
    for k, v in data.items():
        s += data[k][i]
    return s/len(data)

def avg_scale_nodes(i, data, nodes):
    s = 0.0
    num_scale = 0
    for k, v in nodes.items():
        if v > 0:
            num_scale += 1

    for k, v in data.items():
        if nodes[k] > 0:
            s += data[k][i]
    return s/num_scale

def avg_side_nodes(i, data, nodes):
    s = 0.0
    num_side = 0
    for k, v in nodes.items():
        if v == 0:
            num_side += 1

    for k, v in data.items():
        if nodes[k] == 0:
            s += data[k][i]
    return s/num_side

def coll_delay(delay, is_side):
    net_propose_delay = {} 
    all_propose_delay = []

    for k,v in nodes.items():
        if is_side:
            if v == 0: # side nodes
                prev = None
                times = []
                for curr in delay[k]:
                    diff = 0
                    if prev != None:
                        diff = curr - prev
                        prev = curr
                    else:
                        diff = curr
                        prev = curr

                    if diff != 0:
                        times.append(diff)
                        all_propose_delay.append(diff)
                net_propose_delay[k] = times
        else:
            if v > 0: # side nodes
                prev = None
                times = []
                for curr in delay[k]:
                    diff = 0
                    if prev != None:
                        diff = curr - prev
                        prev = curr
                    else:
                        diff = curr
                        prev = curr

                    if diff != 0:
                        times.append(diff)
                        all_propose_delay.append(diff)
                net_propose_delay[k] = times

    return net_propose_delay, all_propose_delay

if len(sys.argv) <3:
    print("need data dir, output name prefix")
    sys.exit(0)



files = {}
nodes_f = "../nodes.txt"
nodes = {}
with open(nodes_f) as f:
    for line in f:
        line = line.replace('\n', '')
        tokens = line.split(',')
        name = tokens[0]
        scale_id = int(tokens[5])
        nodes[name] = scale_id
print(nodes)

# parse data
ts = {}
gen_tx = {}
confirm_tx = {}
propose_delay = {}
propose_num = {}
sign_delay = {}
sign_num = {}
submit_delay = {}
submit_num = {}
is_node_scale ={}
gas = {}
dur = {}
chain_len = {}

data_dir = sys.argv[1] #"../logData-10-5"
directory = sys.argv[2]


duration = 0 #sec
for k,v in nodes.items():
    fname = data_dir + "/" + k + ".txt"
    gen_tx[k] = []
    confirm_tx[k] = []
    propose_delay[k] = [] 
    propose_num[k] = [] 
    sign_delay[k] = []
    sign_num[k] = []  
    submit_delay[k] = [] 
    submit_num[k] = []  
    ts[k] = []
    gas[k] = []
    dur[k] = []
    chain_len[k] = []
    with open(fname) as f:
        for line in f: 
            tokens = line.split(',')[:-1]
            tokens = list(map(float, tokens))
            ts[k].append(tokens[0])
            gen_tx[k].append(tokens[1])
            confirm_tx[k].append(tokens[2])
            propose_delay[k].append(tokens[8]/1000.0)
            sign_delay[k].append(tokens[9]/1000.0)
            submit_delay[k].append(tokens[10]/1000.0)
            propose_num[k].append(tokens[12])
            sign_num[k].append(tokens[13])
            submit_num[k].append(tokens[14])
            gas[k].append(tokens[11])
            dur[k].append(tokens[15])
            chain_len[k].append(tokens[3])
            duration += 1
duration = int(duration / len(nodes))
timestamp = [i for i in range(1, duration)]

# get delay distribution
#print(sign_delay['node_3'])

net_propose_delay, all_propose_delay = coll_delay(propose_delay, True)
net_sign_delay, all_sign_delay = coll_delay(sign_delay, False)
net_submit_delay, all_submit_delay = coll_delay(submit_delay, False)
print(gas)

# print(len(all_propose_delay))
# print(len(all_sign_delay))
# print(len(all_submit_delay))
# print(all_sign_delay)

labels = 'Encoding', 'Oracle'#, 'Trusted Chain'
encode_mean = statistics.mean(all_propose_delay)
sign_mean = statistics.mean(all_sign_delay)
submit_mean = statistics.mean(all_submit_delay)

#print(encode_mean, sign_mean, submit_mean)
print('total latency', encode_mean+ sign_mean)

# get tx-rate
gen_tx_rate = []
confirm_tx_rate = []
tx_sum = []
for i in range(1, duration):
    du = avg_side_nodes(i, dur, nodes)
    #gen_tx_rate.append(avg_nodes(i, gen_tx)/du)
    confirm_tx_rate.append(avg_scale_nodes(i, chain_len, nodes)*13272.0/du)
    #tx_sum.append(avg_side_nodes(i, confirm_tx, nodes))

second_half_tx = confirm_tx_rate[int(len(confirm_tx_rate)/2):]
print("confirm tx", statistics.mean(second_half_tx))
for k,v in chain_len.items():
    print(k, "chain len",v[-1])



if not os.path.exists(directory):
    os.makedirs(directory)
# else:
    # print("dir exist: ", directory)
    # sys.exit(0)

total = float(encode_mean + sign_mean)

sizes = [int(encode_mean/total*100.0), int(sign_mean/total*100.0)]
explode = (0, 0.1)  # only "explode" the 2nd slice (i.e. 'Hogs')

fig1, ax1 = plt.subplots()
ax1.pie(sizes, explode=explode, labels=labels, autopct='%1.1f%%',
        shadow=True, startangle=90)
ax1.axis('equal')  # Equal aspect ratio ensures that pie is drawn as a circle.

plt.savefig(directory + "/pie")

# histogram delay
# fig = plt.figure()
# plt.subplot(3, 1, 1)
# plt.hist(all_propose_delay, bins='auto')
# plt.xlabel("sec")
# plt.title("propose delay")
# plt.subplot(3, 1, 2)
# plt.hist(all_sign_delay, bins='auto')
# plt.xlabel("sec")
# plt.title("sign delay")
# plt.subplot(3, 1, 3)
# plt.hist(all_submit_delay, bins='auto')
# plt.xlabel("sec")
# plt.title("submit delay")
# plt.tight_layout()



# get gas
# net_gas, all_gas = coll_delay(gas, False)
# #print(gas['node_10'])
# a = []
# for i in range(1,11):
    # name = "node_" + str(i)
    # gas_used = net_gas[name]
    # g = []
    # for t in gas_used:
        # if int(t) > 40000:
            # g.append(t)
            # a.append(t)
    # print(g)
# print(len(a))
# #plt.plot(gas['node_10'])
# plt.show()



# plot tx-rate
fig = plt.figure()
# plt.subplot(2, 1, 1)
# plt.plot(timestamp, gen_tx_rate)
# plt.xlabel('time:sec')
# plt.ylabel('tx/sec')
# plt.title('gen_tx_rate')
# plt.subplot(2, 1, 2)
plt.plot(timestamp, confirm_tx_rate)
plt.xlabel('time:sec')
plt.ylabel('tx/sec')
plt.title('confirm_tx_rate')
plt.tight_layout()
plt.savefig(directory + "/tx")

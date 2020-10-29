#!/usr/bin/python3
import sys
import json
import networkx as nx

if len(sys.argv) < 1:
    print('need number of node to build a complete graph')
    sys.exit(0)

num_nodes = int(sys.argv[1])

nodes = []
connections = []

graph = nx.complete_graph(num_nodes)

sys.stderr.write('diameter:'+str(nx.algorithms.distance_measures.diameter(graph))+'\n')
sys.stderr.write('avg_short_path:'+str(nx.average_shortest_path_length(graph))+'\n')

for node in graph.nodes():
    name = "node_" + str(node+1)
    nodes.append(name)
for edge in graph.edges():
    src = "node_" + str(edge[0]+1)
    dst = "node_" + str(edge[1]+1)
    connections.append({
        "from": src,
        "to": dst,
    })
    connections.append({
        "from": dst,
        "to": src,
    })
result = {"nodes": nodes, "connections": connections}
print(json.dumps(result, sort_keys=True, indent=4))

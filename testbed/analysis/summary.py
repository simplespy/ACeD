#!/usr/bin/env python

import matplotlib.pyplot as plt
import sys
import os
import math

fig, axes= plt.subplots(nrows=4, ncols=2) #((ax1, ax2), (ax3, ax4), (ax5, ax6), (ax7, ax8)) 
scat_size = 4
# side 5 dur 5 
cols = ["Throughput (tx/sec)", "Latency (sec)"]
rows = ["A ", "B ", "C ", "D "]

highest_tx = 0
highest_lat = 0



font = 8
ax1 = axes[0,0]
ax2 = axes[0,1]
ax3 = axes[1,0]
ax4 = axes[1,1]
ax5 = axes[2,0]
ax6 = axes[2,1]
ax7 = axes[3,0]
ax8 = axes[3,1]

def plot_bar(ax, x, y, b):
    #low = min(y)
    #high = max(y)

    y_ticks = []
    if b == highest_tx:
        y_ticks = [ (i+1)*10000/4 for i in range(4)]
    else:
        y_ticks = [ (i+1)*5.0/4 for i in range(4)]

    ax.set_ylim(0, b)
    ax.set_yticks(y_ticks)
    ax.plot(x,y, '-o', markersize=3)
    # ax.set_ylim([math.ceil(low-0.5*(high-low)), math.ceil(high+0.5*(high-low))])

    # x_pos = [i for i, _ in enumerate(x)] 
    # ax.bar(x_pos, y)
    # #ax.set_xtick(x_pos,[0] +  x)
    # ax.set_xticks(x_pos, minor=False)
    # ax.set_xticklabels( x)

all_tx = [2758, 2762, 2781, 2730] + [2697, 2797.4, 2745] + [2594.9, 5307, 10513] + [1672,2620,4247.34, 5291]
all_lat = [1.82, 1.70, 2.28, 2.11] + [1.79, 2.4, 3.84] +  [1.75, 3.12, 5.65] + [2.104, 2.91, 2.16, 2.77]
highest_tx = max(all_tx)*1.1
highest_lat = max(all_lat)*1.1



oracles = [5,10,15,25]
tx = [2758, 2762, 2781, 2730]
latency = [1.82, 1.70, 2.28, 2.11]
ax1.tick_params(axis='both', which='major', labelsize=font)
ax1.set_xlabel('# oracles', fontsize=font)
#ax1.set_ylabel('tx/sec', fontsize=font)
#ax1.plot(oracles, tx, '-o')
plot_bar(ax1, oracles, tx, highest_tx)
# x_pos = [i for i, _ in enumerate(oracles)] 
# ax1.bar(x_pos, tx)
# ax1.set_xticklabels([0] + oracles)


ax2.tick_params(axis='both', which='major', labelsize=font)
#ax2.set_title("time=5, num_side=5, vary oracle", fontsize=font)
ax2.set_xlabel("# oracles", fontsize=font)
#ax2.set_ylabel("sec", fontsize=font)
#ax2.plot(oracles, latency, '-o')
plot_bar(ax2, oracles, latency, highest_lat)

sides = [5, 10, 20]
tx = [2697, 2797.4, 2745]
lat = [1.79, 2.4, 3.84]
ax3.tick_params(axis='both', which='major', labelsize=font)
#ax3.set_title("time=5, num_oracle=10, vary side", fontsize=font)
ax3.set_xlabel("# sides", fontsize=font)
#ax3.set_ylabel("tx/sec", fontsize=font)
#ax3.plot(sides, tx, '-o')
plot_bar(ax3, sides, tx, highest_tx)


ax4.tick_params(axis='both', which='major', labelsize=font)
#ax4.set_title("time=5, num_oracle=10, vary side", fontsize=font)
ax4.set_xlabel("# sides", fontsize=font)
#ax4.set_ylabel("sec", fontsize=font)
#ax4.plot(sides, lat, '-o')
plot_bar(ax4, sides, lat, highest_lat)

size = [4, 8, 16]
tx = [2594.9, 5307, 10513]
lat = [1.75, 3.12, 5.65]
ax5.tick_params(axis='both', which='major', labelsize=font)
#ax5.set_title("time=5, num_oracle=5, side=5, vary blk size", fontsize=font)
ax5.set_xlabel("MB", fontsize=font)
#ax5.set_ylabel("tx/sec", fontsize=font)
#ax5.plot(size, tx, '-o')
plot_bar(ax5, size, tx, highest_tx)



ax6.tick_params(axis='both', which='major', labelsize=font)
#ax6.set_title("time=5, num_oracle=5, side=5, vary blk size", fontsize=font)
ax6.set_xlabel("MB", fontsize=font)
#ax6.set_ylabel("sec", fontsize=font)
#ax6.plot(size, lat, '-o')
plot_bar(ax6, size, lat, highest_lat)


time = [8.33, 5, 3.125, 2.5]
tx = [1672,2620,4247.34, 5291]
lat = [2.104, 2.91, 2.16, 2.77]

ax7.tick_params(axis='both', which='major', labelsize=font)
#ax7.set_title("time*num_side=25, num_oracle=10, vary time", fontsize=font)
ax7.set_xlabel("sec", fontsize=font)
#ax7.set_ylabel("tx/sec", fontsize=font)
#ax7.plot(time, tx, '-o')
plot_bar(ax7, time, tx, highest_tx)


#ax8.set_title("time*num_side=25, num_oracle=10, vary time", fontsize=font)
ax8.set_xlabel("sec", fontsize=font)
#ax8.set_ylabel("sec/blk", fontsize=font)
ax8.tick_params(axis='both', which='major', labelsize=font)
#ax8.plot(time, lat, '-o')
plot_bar(ax8, time, lat, highest_lat)

for ax, col in zip(axes[0], cols):
    ax.set_title(col, fontsize=12)

for ax, row in zip(axes[:,0], rows):
    ax.set_ylabel(row, rotation=0, fontsize=12)

plt.tight_layout()


plt.show()

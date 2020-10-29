# ACeD
Authenticated Coded Dispersal (ACeD) is a scalable data availability oracle that interfaces between the side blockchains and the trusted blockchain. It accepts blocks from side blockchains, pushes verifiable commitments to the trusted blockchain and ensures data availability to the side blockchains.

##Paper
This repo includes the prototype implementation evaluated in our *ACeD: Scalable Data Availability Oracle* paper. 

* Full paper: 

##Build Requirements

```
# RUST cargo >= 1.46.0 
# Golang >= 1.13.8
# ethabi developed by paritytech 
# (https://github.com/openethereum/ethabi), 
# please run via cargo:  
cargo install ethabi-cli 

# or run via homebrew:
brew install ethabi  

# Truffle suite >= v5.1.31 (for deploying contract)
# Vultr Clound account (for deploying instance)
# Vultr-cli (for controlling instance)
# s3cmd (for communicating block storage)
# rrdtool (for telematics)
# Ethereum Account: created with MetaMask
# Infura: Ethereum API server
```

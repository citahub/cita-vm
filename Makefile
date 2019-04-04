testdata:
	cd /tmp/ && git clone https://github.com/ethereum/tests jsondata && cd jsondata && git checkout 74cc22b8f

.PHONY: testdata

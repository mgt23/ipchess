.PHONY:

lint: .PHONY
	go vet ./...

gen: .PHONY
	rm -rf gen
	mkdir gen
	protoc --go_out=. ./proto/ipchess.proto
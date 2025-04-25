.PHONY: all clean

all: ec xor

ec: ec.cpp test.h
	g++ -g -O3 -march=native -o ec ec.cpp -lisal

xor: xor.cpp test.h
	g++ -g -O3 -march=native -o xor xor.cpp -lisal

clean:
	rm -f ec xor

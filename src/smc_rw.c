#if defined(__x86_64__)

#include <sys/io.h>

void port_outb(unsigned char value, unsigned short port) {
	outb(value, port);
}

unsigned char port_inb(unsigned short port) {
	return inb(port);
}

int port_ioperm(unsigned long from, unsigned long num, int turn_on) {
	return ioperm(from, num, turn_on);
}

#else

void port_outb(unsigned char, unsigned short) {
}

unsigned char port_inb(unsigned short) {
	return 0;
}

int port_ioperm(unsigned long, unsigned long, int) {
	return 0;
}

#endif

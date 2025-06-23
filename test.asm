section .data
	newline db 10
	format db "%ld", 0
section .text
	global main
	extern printf
	extern exit

main:
	mov rax, 1
	add rax, 2
	mov r8, rax
	mov qword [x], r8
	mov rax, [x]
	add rax, 42
	mov r8, rax
	push rsi
	push rdi
	mov rsi, r8
	mov rdi, format
	xor rax, rax
	call printf
	mov rax, [x]
	cqo
	push rcx
	mov rcx, 6
	idiv rcx
	pop rcx
	mov r8, rax
	mov rax, r8
	cqo
	push rcx
	mov rcx, 5
	idiv rcx
	mov rax, rdx
	pop rcx
	mov r8, rax
	mov qword [y], r8
	mov edi, 0
	call exit

section .bss
	x resq 1
	y resq 1

section .note.GNU-stack noalloc noexec nowrite progbits
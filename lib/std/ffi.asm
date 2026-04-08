section .text
        global c_call
        global c_call_arr

; Calls a C ABI function with up to 4 args and 1 return value.
; Usage call_c function_ptr, args: res;
c_call:
; func_ptr in rdi, args in rsi, rdx, rcx, r8, r9
        push rbp
        mov rbp, rsp
        mov r10, rdi

; Move args into regs expected by C
        mov rdi, rsi
        mov rsi, rdx
        mov rdx, rcx
        mov rcx, r8
        mov r8, r9

        xor rax, rax

        and rsp, -16
        call r10

        mov rsp, rbp
        pop rbp
        ret

; Calls a C ABI function with up to n args and 1 return value.
; Args are passed as an array of ptrs. This array is created by arr in std/array.lang
; Usage call_c function_ptr, args: res;
c_call_arr:
; func_ptr in rdi, args in rsi
        push rbp
        mov rbp, rsp
        push r12

        mov r10, rdi
        mov r12, rsi

        ; n args
        mov r11, [r12 - 8]

        ; if > 6 args, we need to put some on the stack
        cmp r11, 6
        jle .load_regs

        ; stack alignment based on arg number pushed to stack
        test r11, 1
        jz .push_loop_setup
        sub rsp, 8

.push_loop_setup:
        mov rax, r11

.push_loop:
; push all but 6 args to stack
        dec rax
        cmp rax, 5
        jle .load_regs

        ; double deref ptr to arg in array and push
        mov r9, [r12 + rax * 8]
        mov r9, [r9]
        push r9
        jmp .push_loop

.load_regs:
; put remaining <= 6 args in regs
        cmp r11, 0
        jle .do_call
        mov rax, [r12 + 0]
        mov rdi, [rax]

        cmp r11, 1
        jle .do_call
        mov rax, [r12 + 8]
        mov rsi, [rax]

        cmp r11, 2
        jle .do_call
        mov rax, [r12 + 16]
        mov rdx, [rax]

        cmp r11, 3
        jle .do_call
        mov rax, [r12 + 24]
        mov rcx, [rax]

        cmp r11, 4
        jle .do_call
        mov rax, [r12 + 32]
        mov r8, [rax]

        cmp r11, 5
        jle .do_call
        mov rax, [r12 + 40]
        mov r9, [rax]

.do_call:
        xor rax, rax
        call r10

        ; clean up stack
        lea rsp, [rbp - 16]
        pop r13
        pop rbp
        ret


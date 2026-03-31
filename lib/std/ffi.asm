section .text
        global call_c

; Calls a C ABI function with up to 4 args and 1 return value.
; Usage call_c function_ptr, result, args
call_c:
; func_ptr in rdi, result_ptr in rsi, args in rdx, rcx, r8, r9
        push rbp
        push r12
        mov rbp, rsp
        mov r12, rsi
        mov r10, rdi

; Move args into regs expected by C
        mov rdi, rdx
        mov rsi, rcx
        mov rdx, r8
        mov rcx, r9

        xor rax, rax

        and rsp, -16
        call r10

; store result into result_ptr
        mov [r12], rax

        mov rsp, rbp
        pop r12
        pop rbp
        ret

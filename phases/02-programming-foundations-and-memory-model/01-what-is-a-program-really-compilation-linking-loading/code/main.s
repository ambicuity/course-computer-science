	.section	__TEXT,__text,regular,pure_instructions
	.build_version macos, 26, 0	sdk_version 26, 2
	.globl	_main                           ; -- Begin function main
	.p2align	2
_main:                                  ; @main
	.cfi_startproc
; %bb.0:
	sub	sp, sp, #80
	stp	x29, x30, [sp, #64]             ; 16-byte Folded Spill
	add	x29, sp, #64
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	stur	wzr, [x29, #-4]
	stur	w0, [x29, #-8]
	stur	x1, [x29, #-16]
	mov	w8, #7                          ; =0x7
	stur	w8, [x29, #-20]
	mov	x0, #4                          ; =0x4
	bl	_malloc
	str	x0, [sp, #32]
	ldr	x8, [sp, #32]
	cbnz	x8, LBB0_2
	b	LBB0_1
LBB0_1:
	mov	w8, #1                          ; =0x1
	stur	w8, [x29, #-4]
	b	LBB0_7
LBB0_2:
	ldr	x9, [sp, #32]
	mov	w8, #99                         ; =0x63
	str	w8, [x9]
	mov	x9, sp
	adrp	x8, l_.str@PAGE
	add	x8, x8, l_.str@PAGEOFF
	str	x8, [sp, #16]                   ; 8-byte Folded Spill
	str	x8, [x9]
	adrp	x0, l_.str.1@PAGE
	add	x0, x0, l_.str.1@PAGEOFF
	bl	_printf
	adrp	x8, _initialized_global@PAGE
	adrp	x10, _initialized_global@PAGE
	add	x10, x10, _initialized_global@PAGEOFF
	ldr	w8, [x8, _initialized_global@PAGEOFF]
                                        ; kill: def $x8 killed $w8
	mov	x9, sp
	str	x10, [x9]
	str	x8, [x9, #8]
	adrp	x0, l_.str.2@PAGE
	add	x0, x0, l_.str.2@PAGEOFF
	bl	_printf
	adrp	x10, _uninitialized_global@GOTPAGE
	ldr	x10, [x10, _uninitialized_global@GOTPAGEOFF]
	ldr	w8, [x10]
                                        ; kill: def $x8 killed $w8
	mov	x9, sp
	str	x10, [x9]
	str	x8, [x9, #8]
	adrp	x0, l_.str.3@PAGE
	add	x0, x0, l_.str.3@PAGEOFF
	bl	_printf
	ldr	x8, [sp, #16]                   ; 8-byte Folded Reload
	mov	x9, sp
	mov	x10, x8
	str	x10, [x9]
	str	x8, [x9, #8]
	adrp	x0, l_.str.4@PAGE
	add	x0, x0, l_.str.4@PAGEOFF
	bl	_printf
	sub	x10, x29, #20
	ldur	w8, [x29, #-20]
                                        ; kill: def $x8 killed $w8
	mov	x9, sp
	str	x10, [x9]
	str	x8, [x9, #8]
	adrp	x0, l_.str.5@PAGE
	add	x0, x0, l_.str.5@PAGEOFF
	bl	_printf
	ldr	x10, [sp, #32]
	ldr	x8, [sp, #32]
	ldr	w8, [x8]
                                        ; kill: def $x8 killed $w8
	mov	x9, sp
	str	x10, [x9]
	str	x8, [x9, #8]
	adrp	x0, l_.str.6@PAGE
	add	x0, x0, l_.str.6@PAGEOFF
	bl	_printf
	str	wzr, [sp, #28]
	b	LBB0_3
LBB0_3:                                 ; =>This Inner Loop Header: Depth=1
	ldr	w8, [sp, #28]
	subs	w8, w8, #3
	b.ge	LBB0_6
	b	LBB0_4
LBB0_4:                                 ;   in Loop: Header=BB0_3 Depth=1
	bl	_increment_static_local
	mov	x9, sp
                                        ; implicit-def: $x8
	mov	x8, x0
	str	x8, [x9]
	adrp	x0, l_.str.7@PAGE
	add	x0, x0, l_.str.7@PAGEOFF
	bl	_printf
	b	LBB0_5
LBB0_5:                                 ;   in Loop: Header=BB0_3 Depth=1
	ldr	w8, [sp, #28]
	add	w8, w8, #1
	str	w8, [sp, #28]
	b	LBB0_3
LBB0_6:
	ldr	x0, [sp, #32]
	bl	_free
	stur	wzr, [x29, #-4]
	b	LBB0_7
LBB0_7:
	ldur	w0, [x29, #-4]
	ldp	x29, x30, [sp, #64]             ; 16-byte Folded Reload
	add	sp, sp, #80
	ret
	.cfi_endproc
                                        ; -- End function
	.p2align	2                               ; -- Begin function increment_static_local
_increment_static_local:                ; @increment_static_local
	.cfi_startproc
; %bb.0:
	adrp	x8, _increment_static_local.count@PAGE
	ldr	w9, [x8, _increment_static_local.count@PAGEOFF]
	add	w9, w9, #1
	str	w9, [x8, _increment_static_local.count@PAGEOFF]
	ldr	w0, [x8, _increment_static_local.count@PAGEOFF]
	ret
	.cfi_endproc
                                        ; -- End function
	.section	__DATA,__data
	.globl	_initialized_global             ; @initialized_global
	.p2align	2, 0x0
_initialized_global:
	.long	42                              ; 0x2a

	.section	__TEXT,__cstring,cstring_literals
l_.str:                                 ; @.str
	.asciz	"hello, world"

	.section	__DATA,__const
	.globl	_message                        ; @message
	.p2align	3, 0x0
_message:
	.quad	l_.str

	.section	__TEXT,__cstring,cstring_literals
l_.str.1:                               ; @.str.1
	.asciz	"%s\n"

l_.str.2:                               ; @.str.2
	.asciz	"initialized_global    @ %p  = %d\n"

l_.str.3:                               ; @.str.3
	.asciz	"uninitialized_global  @ %p  = %d  (zero by default)\n"

	.comm	_uninitialized_global,4,2       ; @uninitialized_global
l_.str.4:                               ; @.str.4
	.asciz	"message (rodata)      @ %p  = \"%s\"\n"

l_.str.5:                               ; @.str.5
	.asciz	"stack_var             @ %p  = %d  (on the stack)\n"

l_.str.6:                               ; @.str.6
	.asciz	"heap_var               points to %p  = %d  (on the heap)\n"

l_.str.7:                               ; @.str.7
	.asciz	"increment_static_local() = %d  (static keeps state across calls)\n"

.zerofill __DATA,__bss,_increment_static_local.count,4,2 ; @increment_static_local.count
.subsections_via_symbols

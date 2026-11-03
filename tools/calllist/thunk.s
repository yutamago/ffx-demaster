# cdecl int thunk(void *w): runs the call list at thunk+0x100:
# records { u32 fn (absolute, 0 ends), u32 n, u32 args[n] }, args pushed right to left.
# A record with fn == 1 copies: { 1, 2, dst_abs, src_offset_in_w } -> *dst = *(u32*)(w + src)
# A record with fn == 2 stores:  { 2, 2, dst_abs, value } -> *dst = value
# A record with fn == 3 calls:   { 3, 1, f } -> f(w)
# Assemble: as --32 thunk.s -o thunk.o && objcopy -O binary thunk.o thunk.bin (tools/calllist/src/main.rs THUNK)
	.intel_syntax noprefix
	.text
	.globl thunk
thunk:
	push ebp
	mov ebp, esp
	push esi
	push edi
	push ebx
	call 1f
1:	pop esi
	add esi, 0x100 - 0xb
2:	mov eax, [esi]
	test eax, eax
	jz 9f
	cmp eax, 1
	jne 6f
	mov edx, [ebp + 8]
	mov ecx, [esi + 12]
	mov ecx, [edx + ecx]
	mov edx, [esi + 8]
	mov [edx], ecx
	add esi, 16
	jmp 2b
6:	cmp eax, 2
	jne 7f
	mov ecx, [esi + 12]
	mov edx, [esi + 8]
	mov [edx], ecx
	add esi, 16
	jmp 2b
7:	cmp eax, 3
	jne 3f
	push dword ptr [ebp + 8]
	call dword ptr [esi + 8]
	add esp, 4
	add esi, 12
	jmp 2b
3:	mov ebx, [esi + 4]
	mov ecx, ebx
	lea edi, [esi + 4 + ecx*4]
4:	test ecx, ecx
	jz 5f
	push dword ptr [edi]
	sub edi, 4
	dec ecx
	jmp 4b
5:	call eax
	lea esp, [esp + ebx*4]
	lea esi, [esi + 8 + ebx*4]
	jmp 2b
9:	pop ebx
	pop edi
	pop esi
	pop ebp
	xor eax, eax
	ret

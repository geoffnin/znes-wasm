; 65816 Addressing Modes Test Program
; Tests all addressing modes with read and write operations
; Results stored at $7E0100-$7E01FF

.cpu "65816"

; Result structure: test_id, expected, actual, pass_flag
.org $8000

RESET:
    sei                     ; Disable interrupts
    clc                     ; Clear carry for native mode
    xce                     ; Switch to native mode
    
    ; Set up registers to 8-bit mode
    sep #$30                ; Set A, X, Y to 8-bit
    
    ; Initialize Direct Page to $0000 first
    rep #$20                ; 16-bit A
    lda #$0000
    tcd                     ; Transfer to Direct Page
    sep #$20                ; Back to 8-bit A
    
    ; Clear result area
    jsr clear_results
    
    ; Run all addressing mode tests
    jsr test_immediate
    jsr test_direct_page
    jsr test_direct_page_indexed
    jsr test_absolute
    jsr test_absolute_indexed
    jsr test_absolute_long
    jsr test_dp_indirect
    jsr test_dp_indexed_indirect
    jsr test_dp_indirect_indexed
    jsr test_absolute_indexed_indirect
    jsr test_stack_relative
    jsr test_stack_relative_indirect_indexed
    
    ; Test with different Direct Page values
    jsr test_with_dp_2000
    jsr test_with_dp_ff00
    
    ; Test boundary conditions
    jsr test_page_crossing
    jsr test_bank_wrapping
    
    ; Infinite loop
END:
    jmp END

;==============================================================================
; UTILITY FUNCTIONS
;==============================================================================

clear_results:
    rep #$30                ; 16-bit A, X, Y
    ldx #$0000
.loop:
    stz $7E0100,x          ; Clear result area
    inx
    inx
    cpx #$0100
    bne .loop
    sep #$30                ; Back to 8-bit
    rts

; Store test result
; A = test_id, X = expected, Y = actual
store_result:
    pha                     ; Save test_id
    phx                     ; Save expected
    phy                     ; Save actual
    
    rep #$20                ; 16-bit A
    and #$00FF              ; Clear high byte
    asl a                   ; Multiply by 4
    asl a
    tax                     ; X = offset
    sep #$20                ; 8-bit A
    
    pla                     ; Get test_id
    sta $7E0100,x          ; Store test_id
    
    pla                     ; Get expected
    sta $7E0101,x          ; Store expected
    
    pla                     ; Get actual
    sta $7E0102,x          ; Store actual
    
    ; Check if pass
    lda $7E0101,x
    cmp $7E0102,x
    bne .fail
    lda #$01
    bra .store_flag
.fail:
    lda #$00
.store_flag:
    sta $7E0103,x          ; Store pass_flag
    rts

;==============================================================================
; TEST 1: IMMEDIATE ADDRESSING
; Format: LDA #$12 (8-bit) or LDA #$1234 (16-bit)
; Loads immediate value into register
;==============================================================================

test_immediate:
    ; Test 8-bit immediate
    lda #$42                ; Load immediate value $42
    ldx #$42                ; Expected
    tay                     ; Actual
    lda #$01                ; Test ID 1
    jsr store_result
    
    ; Test 16-bit immediate
    rep #$20                ; 16-bit A
    lda #$1234              ; Load immediate value $1234
    sep #$20                ; 8-bit A
    xba                     ; Swap bytes to get high byte
    ldx #$12                ; Expected high byte
    tay                     ; Actual high byte
    lda #$02                ; Test ID 2
    jsr store_result
    
    xba                     ; Swap back to get low byte
    ldx #$34                ; Expected low byte
    tay                     ; Actual low byte
    lda #$03                ; Test ID 3
    jsr store_result
    
    rts

;==============================================================================
; TEST 2: DIRECT PAGE ADDRESSING
; Format: LDA $12
; Address = Direct Page + $12
;==============================================================================

test_direct_page:
    ; Set up test data at DP+$12 (currently $0000+$12 = $0012)
    lda #$55
    sta $12                 ; Store $55 at DP+$12
    
    ; Test read
    lda $12                 ; Load from DP+$12, should get $55
    ldx #$55                ; Expected
    tay                     ; Actual
    lda #$04                ; Test ID 4
    jsr store_result
    
    ; Test write
    lda #$AA
    sta $12                 ; Store $AA at DP+$12
    lda $12                 ; Read back
    ldx #$AA                ; Expected
    tay                     ; Actual
    lda #$05                ; Test ID 5
    jsr store_result
    
    rts

;==============================================================================
; TEST 3: DIRECT PAGE INDEXED
; Format: LDA $12,X or LDA $12,Y
; Address = Direct Page + $12 + X (or Y)
;==============================================================================

test_direct_page_indexed:
    ; Set up test data
    lda #$33
    sta $20                 ; Store $33 at DP+$20
    lda #$44
    sta $25                 ; Store $44 at DP+$25
    
    ; Test $12,X with X=$0E (DP+$12+$0E = DP+$20)
    ldx #$0E
    lda #$33
    sta $12,x               ; Store at DP+$20
    lda $12,x               ; Load from DP+$20
    ldx #$33                ; Expected
    tay                     ; Actual
    lda #$06                ; Test ID 6
    jsr store_result
    
    ; Test $12,Y with Y=$13 (DP+$12+$13 = DP+$25)
    ldy #$13
    lda #$44
    sta $12,y               ; Store at DP+$25
    lda $12,y               ; Load from DP+$25
    ldx #$44                ; Expected
    tay                     ; Actual
    lda #$07                ; Test ID 7
    jsr store_result
    
    rts

;==============================================================================
; TEST 4: ABSOLUTE ADDRESSING
; Format: LDA $1234
; Address = $1234 (in current data bank)
;==============================================================================

test_absolute:
    ; Set up test data at $7E:1234
    lda #$66
    sta $7E1234             ; Store using long addressing
    
    ; Test read using absolute (assumes DB=$7E)
    phb                     ; Save data bank
    lda #$7E
    pha
    plb                     ; Set DB=$7E
    
    lda $1234               ; Load from $7E:1234
    ldx #$66                ; Expected
    tay                     ; Actual
    lda #$08                ; Test ID 8
    jsr store_result
    
    ; Test write
    lda #$77
    sta $1234               ; Store to $7E:1234
    lda $1234               ; Read back
    ldx #$77                ; Expected
    tay                     ; Actual
    lda #$09                ; Test ID 9
    jsr store_result
    
    plb                     ; Restore data bank
    rts

;==============================================================================
; TEST 5: ABSOLUTE INDEXED
; Format: LDA $1234,X or LDA $1234,Y
; Address = $1234 + X (or Y) in current data bank
;==============================================================================

test_absolute_indexed:
    phb                     ; Save data bank
    lda #$7E
    pha
    plb                     ; Set DB=$7E
    
    ; Set up test data
    lda #$88
    sta $1240               ; Store at $7E:1240
    lda #$99
    sta $1250               ; Store at $7E:1250
    
    ; Test $1234,X with X=$0C ($1234+$0C = $1240)
    ldx #$0C
    lda $1234,x             ; Load from $7E:1240
    ldx #$88                ; Expected
    tay                     ; Actual
    lda #$0A                ; Test ID 10
    jsr store_result
    
    ; Test $1234,Y with Y=$1C ($1234+$1C = $1250)
    ldy #$1C
    lda $1234,y             ; Load from $7E:1250
    ldx #$99                ; Expected
    tay                     ; Actual
    lda #$0B                ; Test ID 11
    jsr store_result
    
    plb                     ; Restore data bank
    rts

;==============================================================================
; TEST 6: ABSOLUTE LONG ADDRESSING
; Format: LDA $123456
; Address = $123456 (full 24-bit address)
;==============================================================================

test_absolute_long:
    ; Set up test data at $7E:2000
    lda #$AB
    sta $7E2000             ; Store using long addressing
    
    ; Test read
    lda $7E2000             ; Load using long addressing
    ldx #$AB                ; Expected
    tay                     ; Actual
    lda #$0C                ; Test ID 12
    jsr store_result
    
    ; Test write
    lda #$CD
    sta $7E2000             ; Store using long addressing
    lda $7E2000             ; Read back
    ldx #$CD                ; Expected
    tay                     ; Actual
    lda #$0D                ; Test ID 13
    jsr store_result
    
    rts

;==============================================================================
; TEST 7: DIRECT PAGE INDIRECT
; Format: LDA ($12)
; Address = [Direct Page + $12] (16-bit pointer at DP+$12)
;==============================================================================

test_dp_indirect:
    phb                     ; Save data bank
    lda #$7E
    pha
    plb                     ; Set DB=$7E
    
    ; Set up pointer at DP+$30 pointing to $2100
    rep #$20                ; 16-bit A
    lda #$2100
    sta $30                 ; Store pointer at DP+$30
    sep #$20                ; 8-bit A
    
    ; Set up test data at $7E:2100
    lda #$EF
    sta $2100
    
    ; Test read using ($30)
    lda ($30)               ; Load from address in pointer at DP+$30
    ldx #$EF                ; Expected
    tay                     ; Actual
    lda #$0E                ; Test ID 14
    jsr store_result
    
    ; Test write
    lda #$FE
    sta ($30)               ; Store to address in pointer
    lda ($30)               ; Read back
    ldx #$FE                ; Expected
    tay                     ; Actual
    lda #$0F                ; Test ID 15
    jsr store_result
    
    plb                     ; Restore data bank
    rts

;==============================================================================
; TEST 8: DIRECT PAGE INDEXED INDIRECT
; Format: LDA ($12,X)
; Address = [Direct Page + $12 + X] (16-bit pointer)
;==============================================================================

test_dp_indexed_indirect:
    phb                     ; Save data bank
    lda #$7E
    pha
    plb                     ; Set DB=$7E
    
    ; Set up pointer at DP+$40 ($30+$10) pointing to $2200
    rep #$20                ; 16-bit A
    lda #$2200
    sta $40                 ; Store pointer at DP+$40
    sep #$20                ; 8-bit A
    
    ; Set up test data at $7E:2200
    lda #$11
    sta $2200
    
    ; Test read using ($30,X) with X=$10
    ldx #$10
    lda ($30,x)             ; Load from address in pointer at DP+$40
    ldx #$11                ; Expected
    tay                     ; Actual
    lda #$10                ; Test ID 16
    jsr store_result
    
    ; Test write
    ldx #$10
    lda #$22
    sta ($30,x)             ; Store to address in pointer
    lda ($30,x)             ; Read back
    ldx #$22                ; Expected
    tay                     ; Actual
    lda #$11                ; Test ID 17
    jsr store_result
    
    plb                     ; Restore data bank
    rts

;==============================================================================
; TEST 9: DIRECT PAGE INDIRECT INDEXED
; Format: LDA ($12),Y
; Address = [Direct Page + $12] + Y
;==============================================================================

test_dp_indirect_indexed:
    phb                     ; Save data bank
    lda #$7E
    pha
    plb                     ; Set DB=$7E
    
    ; Set up pointer at DP+$50 pointing to $2300
    rep #$20                ; 16-bit A
    lda #$2300
    sta $50                 ; Store pointer at DP+$50
    sep #$20                ; 8-bit A
    
    ; Set up test data at $7E:2310 ($2300+$10)
    lda #$33
    sta $2310
    
    ; Test read using ($50),Y with Y=$10
    ldy #$10
    lda ($50),y             ; Load from $2300+$10 = $2310
    ldx #$33                ; Expected
    tay                     ; Actual (save before overwriting Y)
    lda #$12                ; Test ID 18
    jsr store_result
    
    ; Test write
    ldy #$10
    lda #$44
    sta ($50),y             ; Store to $2310
    lda ($50),y             ; Read back
    ldx #$44                ; Expected
    tay                     ; Actual
    lda #$13                ; Test ID 19
    jsr store_result
    
    plb                     ; Restore data bank
    rts

;==============================================================================
; TEST 10: ABSOLUTE INDEXED INDIRECT
; Format: JMP ($1234,X) or JML ($1234,X)
; Address = [Bank:$1234 + X]
; Note: This mode is primarily used for JMP/JSR
;==============================================================================

test_absolute_indexed_indirect:
    phb                     ; Save data bank
    lda #$7E
    pha
    plb                     ; Set DB=$7E
    
    ; Set up pointer at $1260 ($1250+$10) pointing to target
    rep #$20                ; 16-bit A
    lda #.target
    sta $1260               ; Store pointer
    sep #$20                ; 8-bit A
    
    ; We can't directly test JMP, so we test reading the pointer
    ldx #$10
    rep #$20                ; 16-bit A
    lda $1250,x             ; Read pointer value
    sep #$20                ; 8-bit A
    
    ; Check low byte
    ldx #<.target           ; Expected low byte
    tay                     ; Actual low byte
    lda #$14                ; Test ID 20
    jsr store_result
    
    plb                     ; Restore data bank
.target:
    rts

;==============================================================================
; TEST 11: STACK RELATIVE
; Format: LDA $12,S
; Address = Stack Pointer + $12
;==============================================================================

test_stack_relative:
    ; Push test value onto stack
    lda #$55
    pha
    
    ; Access it using stack relative (SP+$01 points to the pushed value)
    lda $01,s               ; Load from SP+$01
    ldx #$55                ; Expected
    tay                     ; Actual
    lda #$15                ; Test ID 21
    jsr store_result
    
    ; Modify using stack relative
    lda #$66
    sta $01,s               ; Store to SP+$01
    lda $01,s               ; Read back
    ldx #$66                ; Expected
    tay                     ; Actual
    lda #$16                ; Test ID 22
    jsr store_result
    
    pla                     ; Clean up stack
    rts

;==============================================================================
; TEST 12: STACK RELATIVE INDIRECT INDEXED
; Format: LDA ($12,S),Y
; Address = [Stack Pointer + $12] + Y
;==============================================================================

test_stack_relative_indirect_indexed:
    phb                     ; Save data bank
    lda #$7E
    pha
    plb                     ; Set DB=$7E
    
    ; Set up test data at $7E:2400
    lda #$77
    sta $2410               ; Store at $2400+$10
    
    ; Push pointer to $2400 onto stack
    rep #$20                ; 16-bit A
    lda #$2400
    pha                     ; Push pointer
    sep #$20                ; 8-bit A
    
    ; Test read using ($01,S),Y with Y=$10
    ldy #$10
    lda ($01,s),y           ; Load from [$2400]+$10 = $2410
    ldx #$77                ; Expected
    tay                     ; Actual
    lda #$17                ; Test ID 23
    jsr store_result
    
    ; Test write
    ldy #$10
    lda #$88
    sta ($01,s),y           ; Store to $2410
    lda ($01,s),y           ; Read back
    ldx #$88                ; Expected
    tay                     ; Actual
    lda #$18                ; Test ID 24
    jsr store_result
    
    pla                     ; Clean up stack (16-bit)
    pla
    plb                     ; Restore data bank
    rts

;==============================================================================
; TEST 13: DIFFERENT DIRECT PAGE VALUES
; Test with DP=$2000
;==============================================================================

test_with_dp_2000:
    ; Save current DP
    rep #$20                ; 16-bit A
    tdc                     ; Transfer DP to A
    pha                     ; Save it
    
    ; Set DP to $2000
    lda #$2000
    tcd                     ; Transfer to DP
    sep #$20                ; 8-bit A
    
    ; Set up test data at $2012 (DP=$2000 + $12)
    lda #$99
    sta $12                 ; Store at DP+$12 = $2012
    
    ; Test read
    lda $12                 ; Should read from $2012
    ldx #$99                ; Expected
    tay                     ; Actual
    lda #$19                ; Test ID 25
    jsr store_result
    
    ; Restore DP
    rep #$20                ; 16-bit A
    pla
    tcd
    sep #$20                ; 8-bit A
    rts

;==============================================================================
; TEST 14: DIFFERENT DIRECT PAGE VALUES
; Test with DP=$FF00
;==============================================================================

test_with_dp_ff00:
    ; Save current DP
    rep #$20                ; 16-bit A
    tdc
    pha
    
    ; Set DP to $FF00
    lda #$FF00
    tcd
    sep #$20                ; 8-bit A
    
    ; Set up test data at $FF12 (DP=$FF00 + $12)
    lda #$AA
    sta $12                 ; Store at DP+$12 = $FF12
    
    ; Test read
    lda $12                 ; Should read from $FF12
    ldx #$AA                ; Expected
    tay                     ; Actual
    lda #$1A                ; Test ID 26
    jsr store_result
    
    ; Restore DP
    rep #$20                ; 16-bit A
    pla
    tcd
    sep #$20                ; 8-bit A
    rts

;==============================================================================
; TEST 15: PAGE CROSSING
; Test when indexed addressing crosses page boundary
;==============================================================================

test_page_crossing:
    phb                     ; Save data bank
    lda #$7E
    pha
    plb                     ; Set DB=$7E
    
    ; Set up test data at $12FF (crosses to $1300)
    lda #$BB
    sta $12FF
    lda #$CC
    sta $1300
    
    ; Test crossing with X
    ldx #$05
    lda #$BB
    sta $12FA,x             ; $12FA+$05 = $12FF
    lda $12FA,x
    ldx #$BB                ; Expected
    tay                     ; Actual
    lda #$1B                ; Test ID 27
    jsr store_result
    
    ; Test crossing boundary
    ldx #$06
    lda #$CC
    sta $12FA,x             ; $12FA+$06 = $1300
    lda $12FA,x
    ldx #$CC                ; Expected
    tay                     ; Actual
    lda #$1C                ; Test ID 28
    jsr store_result
    
    plb                     ; Restore data bank
    rts

;==============================================================================
; TEST 16: BANK WRAPPING
; Test when long addressing wraps within bank
;==============================================================================

test_bank_wrapping:
    ; Set up test data at $7EFFFF and $7E0000
    lda #$DD
    sta $7EFFFF
    lda #$EE
    sta $7E0000
    
    ; Test access near bank boundary
    lda $7EFFFF
    ldx #$DD                ; Expected
    tay                     ; Actual
    lda #$1D                ; Test ID 29
    jsr store_result
    
    lda $7E0000
    ldx #$EE                ; Expected
    tay                     ; Actual
    lda #$1E                ; Test ID 30
    jsr store_result
    
    rts

;==============================================================================
; VECTOR TABLE
;==============================================================================

.org $FFFC
.word RESET                 ; Reset vector
.word $0000                 ; IRQ vector

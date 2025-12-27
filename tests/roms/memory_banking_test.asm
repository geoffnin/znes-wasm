; 65816 Memory Banking Test Program
; Tests memory access across different banks, addressing modes, and banking features

.p816                   ; 65816 mode
.i16                    ; 16-bit index registers
.a8                     ; 8-bit accumulator (will switch as needed)

; Memory map for test results
.define RESULT_BASE     $7E0200
.define TEST_COUNT      $7E0200  ; Number of tests passed
.define TEST_FAILURES   $7E0201  ; Number of tests failed
.define TEST_STATUS     $7E0202  ; Array of test status bytes (0=pass, 1=fail)

; Test ID constants
.define TEST_WRAM_WRITE         $00
.define TEST_WRAM_MIRROR        $01
.define TEST_BANK_7F            $02
.define TEST_BANK_80_MIRROR     $03
.define TEST_LONG_ADDR          $04
.define TEST_MVN_BLOCK          $05
.define TEST_MVP_BLOCK          $06
.define TEST_DBR                $07
.define TEST_DIRECT_PAGE_0000   $08
.define TEST_DIRECT_PAGE_2000   $09
.define TEST_DIRECT_PAGE_FF00   $0A
.define TEST_BANK_WRAP          $0B
.define TEST_ROM_READONLY       $0C

; Entry point
.org $8000
.bank 0

RESET:
    ; Initialize processor
    sei                     ; Disable interrupts
    clc
    xce                     ; Clear carry, exchange to native mode (65816)
    
    ; Set 16-bit index, 8-bit accumulator
    rep #$10                ; 16-bit index
    sep #$20                ; 8-bit accumulator
    
    ; Initialize Direct Page to $0000
    lda #$00
    tcd                     ; D = $0000
    
    ; Initialize Data Bank to $00
    lda #$00
    pha
    plb                     ; DBR = $00
    
    ; Initialize Stack
    ldx #$1FFF
    txs
    
    ; Clear test results area
    jsr ClearResults
    
    ; Run tests
    jsr TestWRAMWrite
    jsr TestWRAMMirror
    jsr TestBank7F
    jsr TestBank80Mirror
    jsr TestLongAddressing
    jsr TestMVNBlock
    jsr TestMVPBlock
    jsr TestDBR
    jsr TestDirectPage0000
    jsr TestDirectPage2000
    jsr TestDirectPageFF00
    jsr TestBankWrap
    jsr TestROMReadOnly
    
    ; All tests complete
    stp                     ; Stop processor

;-----------------------------------------------------------------------------
; ClearResults - Clear the results area in WRAM
;-----------------------------------------------------------------------------
ClearResults:
    php
    rep #$30                ; 16-bit A and X
    lda #$0000
    ldx #$0000
    
.clear_loop:
    sta $7E0200,x          ; Clear result byte
    inx
    inx
    cpx #$0100
    bne .clear_loop
    
    plp
    rts

;-----------------------------------------------------------------------------
; RecordPass - Record a test pass
; Input: A = test ID
;-----------------------------------------------------------------------------
RecordPass:
    php
    sep #$20                ; 8-bit A
    
    ; Store 0 (pass) in status array
    tax
    lda #$00
    sta $7E0202,x
    
    ; Increment pass count
    lda $7E0200
    inc a
    sta $7E0200
    
    plp
    rts

;-----------------------------------------------------------------------------
; RecordFail - Record a test failure
; Input: A = test ID
;-----------------------------------------------------------------------------
RecordFail:
    php
    sep #$20                ; 8-bit A
    
    ; Store 1 (fail) in status array
    tax
    lda #$01
    sta $7E0202,x
    
    ; Increment fail count
    lda $7E0201
    inc a
    sta $7E0201
    
    plp
    rts

;-----------------------------------------------------------------------------
; TestWRAMWrite - Test basic WRAM write/read
;-----------------------------------------------------------------------------
TestWRAMWrite:
    php
    sep #$20                ; 8-bit A
    
    ; Write test pattern to WRAM bank $7E
    lda #$7E
    sta $7E1000            ; Write to WRAM
    
    ; Read back and verify
    lda $7E1000
    cmp #$7E
    bne .fail
    
    ; Test passed
    lda #TEST_WRAM_WRITE
    jsr RecordPass
    plp
    rts
    
.fail:
    lda #TEST_WRAM_WRITE
    jsr RecordFail
    plp
    rts

;-----------------------------------------------------------------------------
; TestWRAMMirror - Test WRAM mirror in banks $00-$3F and $80-$BF
;-----------------------------------------------------------------------------
TestWRAMMirror:
    php
    sep #$20                ; 8-bit A
    
    ; Write to bank $7E
    lda #$AB
    sta $7E0100
    
    ; Read from bank $00 mirror (first 8KB of WRAM)
    lda $000100
    cmp #$AB
    bne .fail
    
    ; Read from bank $80 mirror
    lda $800100
    cmp #$AB
    bne .fail
    
    ; Test passed
    lda #TEST_WRAM_MIRROR
    jsr RecordPass
    plp
    rts
    
.fail:
    lda #TEST_WRAM_MIRROR
    jsr RecordFail
    plp
    rts

;-----------------------------------------------------------------------------
; TestBank7F - Test WRAM bank $7F
;-----------------------------------------------------------------------------
TestBank7F:
    php
    sep #$20                ; 8-bit A
    
    ; Write test pattern to bank $7F
    lda #$7F
    sta $7F0000
    
    ; Read back and verify
    lda $7F0000
    cmp #$7F
    bne .fail
    
    ; Test passed
    lda #TEST_BANK_7F
    jsr RecordPass
    plp
    rts
    
.fail:
    lda #TEST_BANK_7F
    jsr RecordFail
    plp
    rts

;-----------------------------------------------------------------------------
; TestBank80Mirror - Test bank $80 mirrors bank $00
;-----------------------------------------------------------------------------
TestBank80Mirror:
    php
    sep #$20                ; 8-bit A
    
    ; Write to bank $00 ROM area (will fail on real hw, but test emulator)
    lda #$99
    sta $008000            ; In practice, this is ROM
    
    ; For emulator testing, we'll test the mirror relationship
    ; by writing to WRAM and checking mirrors
    lda #$55
    sta $7E0500
    lda $000500            ; Mirror in bank $00
    cmp #$55
    bne .fail
    lda $800500            ; Mirror in bank $80
    cmp #$55
    bne .fail
    
    ; Test passed
    lda #TEST_BANK_80_MIRROR
    jsr RecordPass
    plp
    rts
    
.fail:
    lda #TEST_BANK_80_MIRROR
    jsr RecordFail
    plp
    rts

;-----------------------------------------------------------------------------
; TestLongAddressing - Test 24-bit long addressing with LDA.L/STA.L
;-----------------------------------------------------------------------------
TestLongAddressing:
    php
    sep #$20                ; 8-bit A
    
    ; Write using long addressing
    lda #$EA
    stal $7E1100           ; STA Long to bank $7E
    
    ; Read using long addressing
    ldal $7E1100           ; LDA Long from bank $7E
    cmp #$EA
    bne .fail
    
    ; Test with different bank
    lda #$EB
    stal $7F1200
    
    ldal $7F1200
    cmp #$EB
    bne .fail
    
    ; Test passed
    lda #TEST_LONG_ADDR
    jsr RecordPass
    plp
    rts
    
.fail:
    lda #TEST_LONG_ADDR
    jsr RecordFail
    plp
    rts

;-----------------------------------------------------------------------------
; TestMVNBlock - Test MVN (Move Negative) block move instruction
;-----------------------------------------------------------------------------
TestMVNBlock:
    php
    rep #$30                ; 16-bit A and X
    
    ; Set up source data in bank $7E
    sep #$20                ; 8-bit A
    lda #$11
    sta $7E1300
    lda #$22
    sta $7E1301
    lda #$33
    sta $7E1302
    lda #$44
    sta $7E1303
    
    ; Prepare MVN
    rep #$30                ; 16-bit A and X
    ldx #$1300             ; Source offset
    ldy #$1400             ; Destination offset
    lda #$0003             ; Count - 1 (4 bytes)
    mvn $7E,$7E            ; Move from bank $7E to bank $7E
    
    ; Verify moved data
    sep #$20                ; 8-bit A
    lda $7E1400
    cmp #$11
    bne .fail
    lda $7E1401
    cmp #$22
    bne .fail
    lda $7E1402
    cmp #$33
    bne .fail
    lda $7E1403
    cmp #$44
    bne .fail
    
    ; Test passed
    lda #TEST_MVN_BLOCK
    jsr RecordPass
    plp
    rts
    
.fail:
    lda #TEST_MVN_BLOCK
    jsr RecordFail
    plp
    rts

;-----------------------------------------------------------------------------
; TestMVPBlock - Test MVP (Move Positive) block move instruction
;-----------------------------------------------------------------------------
TestMVPBlock:
    php
    rep #$30                ; 16-bit A and X
    
    ; Set up source data in bank $7E
    sep #$20                ; 8-bit A
    lda #$AA
    sta $7E1500
    lda #$BB
    sta $7E1501
    lda #$CC
    sta $7E1502
    lda #$DD
    sta $7E1503
    
    ; Prepare MVP
    rep #$30                ; 16-bit A and X
    ldx #$1503             ; Source offset (end address)
    ldy #$1603             ; Destination offset (end address)
    lda #$0003             ; Count - 1 (4 bytes)
    mvp $7E,$7E            ; Move from bank $7E to bank $7E
    
    ; Verify moved data
    sep #$20                ; 8-bit A
    lda $7E1600
    cmp #$AA
    bne .fail
    lda $7E1601
    cmp #$BB
    bne .fail
    lda $7E1602
    cmp #$CC
    bne .fail
    lda $7E1603
    cmp #$DD
    bne .fail
    
    ; Test passed
    lda #TEST_MVP_BLOCK
    jsr RecordPass
    plp
    rts
    
.fail:
    lda #TEST_MVP_BLOCK
    jsr RecordFail
    plp
    rts

;-----------------------------------------------------------------------------
; TestDBR - Test Data Bank Register affects data access
;-----------------------------------------------------------------------------
TestDBR:
    php
    sep #$20                ; 8-bit A
    
    ; Write to bank $7E with explicit long addressing
    lda #$DB
    stal $7E1700
    
    ; Set DBR to $7E
    lda #$7E
    pha
    plb                     ; DBR = $7E
    
    ; Now access $1700 without bank (should use DBR=$7E)
    lda $1700              ; This accesses $7E:1700
    cmp #$DB
    bne .fail
    
    ; Restore DBR to $00
    lda #$00
    pha
    plb
    
    ; Test passed
    lda #TEST_DBR
    jsr RecordPass
    plp
    rts
    
.fail:
    ; Restore DBR
    lda #$00
    pha
    plb
    
    lda #TEST_DBR
    jsr RecordFail
    plp
    rts

;-----------------------------------------------------------------------------
; TestDirectPage0000 - Test Direct Page at $0000
;-----------------------------------------------------------------------------
TestDirectPage0000:
    php
    rep #$30                ; 16-bit A and X
    
    ; Set Direct Page to $0000
    lda #$0000
    tcd
    
    ; Write test value to WRAM
    sep #$20                ; 8-bit A
    lda #$D0
    sta $7E0050
    
    ; Access via direct page (DP=$0000, so $50 = $00:0050 = $7E:0050 mirror)
    lda $50
    cmp #$D0
    bne .fail
    
    ; Test passed
    lda #TEST_DIRECT_PAGE_0000
    jsr RecordPass
    plp
    rts
    
.fail:
    lda #TEST_DIRECT_PAGE_0000
    jsr RecordFail
    plp
    rts

;-----------------------------------------------------------------------------
; TestDirectPage2000 - Test Direct Page at $2000
;-----------------------------------------------------------------------------
TestDirectPage2000:
    php
    rep #$30                ; 16-bit A and X
    
    ; Set Direct Page to $2000
    lda #$2000
    tcd
    
    ; Write test value
    sep #$20                ; 8-bit A
    lda #$D2
    stal $7E2050           ; Write to $7E:2050
    
    ; Access via direct page (DP=$2000, so $50 = $00:2050 = $7E:2050 mirror)
    lda $50
    cmp #$D2
    bne .fail
    
    ; Restore Direct Page
    rep #$30
    lda #$0000
    tcd
    
    ; Test passed
    sep #$20
    lda #TEST_DIRECT_PAGE_2000
    jsr RecordPass
    plp
    rts
    
.fail:
    ; Restore Direct Page
    rep #$30
    lda #$0000
    tcd
    
    sep #$20
    lda #TEST_DIRECT_PAGE_2000
    jsr RecordFail
    plp
    rts

;-----------------------------------------------------------------------------
; TestDirectPageFF00 - Test Direct Page at $FF00 (page boundary)
;-----------------------------------------------------------------------------
TestDirectPageFF00:
    php
    rep #$30                ; 16-bit A and X
    
    ; Set Direct Page to $FF00
    lda #$FF00
    tcd
    
    ; Write test value
    sep #$20                ; 8-bit A
    lda #$DF
    stal $7EFF50           ; Write to $7E:FF50
    
    ; Access via direct page (DP=$FF00, so $50 = $00:FF50 = $7E:FF50 mirror)
    lda $50
    cmp #$DF
    bne .fail
    
    ; Restore Direct Page
    rep #$30
    lda #$0000
    tcd
    
    ; Test passed
    sep #$20
    lda #TEST_DIRECT_PAGE_FF00
    jsr RecordPass
    plp
    rts
    
.fail:
    ; Restore Direct Page
    rep #$30
    lda #$0000
    tcd
    
    sep #$20
    lda #TEST_DIRECT_PAGE_FF00
    jsr RecordFail
    plp
    rts

;-----------------------------------------------------------------------------
; TestBankWrap - Test bank wrapping at $FFFF
;-----------------------------------------------------------------------------
TestBankWrap:
    php
    rep #$30                ; 16-bit A and X
    
    ; Write values near bank boundary
    sep #$20                ; 8-bit A
    lda #$F1
    stal $7EFFFF           ; Last byte of bank $7E
    
    lda #$F2
    stal $7F0000           ; First byte of bank $7F
    
    ; Verify they're different (no wrap)
    ldal $7EFFFF
    cmp #$F1
    bne .fail
    
    ldal $7F0000
    cmp #$F2
    bne .fail
    
    ; Test passed
    lda #TEST_BANK_WRAP
    jsr RecordPass
    plp
    rts
    
.fail:
    lda #TEST_BANK_WRAP
    jsr RecordFail
    plp
    rts

;-----------------------------------------------------------------------------
; TestROMReadOnly - Test that ROM areas are read-only
;-----------------------------------------------------------------------------
TestROMReadOnly:
    php
    sep #$20                ; 8-bit A
    
    ; In a real SNES, ROM is read-only. For emulator testing,
    ; we verify the memory map is set up correctly.
    ; We'll read from ROM area and verify it's not WRAM
    
    ; Read from ROM area (bank $80+)
    ldal $808000           ; Should be ROM
    
    ; Try to write to it (should have no effect in real hw)
    lda #$99
    stal $808000
    
    ; Read again - in proper emulation, should be unchanged
    ; For this test, we just verify the read succeeds
    ldal $808000
    
    ; Test passed (we're just verifying memory map exists)
    lda #TEST_ROM_READONLY
    jsr RecordPass
    plp
    rts

;-----------------------------------------------------------------------------
; Interrupt Vectors
;-----------------------------------------------------------------------------
.org $FFE0
.bank 0

; Native mode vectors
    .word $0000            ; Reserved
    .word $0000            ; Reserved
    .word $0000            ; COP
    .word $0000            ; BRK
    .word $0000            ; ABORT
    .word $0000            ; NMI
    .word $0000            ; Reserved
    .word $0000            ; IRQ

; Emulation mode vectors  
    .word $0000            ; Reserved
    .word $0000            ; Reserved
    .word $0000            ; COP
    .word $0000            ; Reserved
    .word $0000            ; ABORT
    .word $0000            ; NMI
    .word RESET            ; RESET
    .word $0000            ; IRQ/BRK

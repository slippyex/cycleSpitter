                lea     charBuffer,a0
                lea     buffer8,a1
                addq.w  #1,delayCounter
.loop:          tst.w d0
                movem.l d0-d7/a1-a3,-(sp)
;---------------------------------------------------------
; SCROLLOOP: Loop that performs the scrolling effect on the bitmap.
;---------------------------------------------------------
                rept 7
                    lsl.w   (a0)+
                    addq.l  #2,a0
add                 set 224
                    rept    28
                        roxl.w  add(a1)
add set add-8
                    endr
                    roxl.w  (a1)
                    lea     SCREEN_WIDTH(a1),a1
                endr
; --------------- safe lea start ----------
                lsl.w   (a0)+
                addq.l  #2,a0
add set 224
                rept    28
                    roxl.w  add(a1)
add set add-8
                endr
                roxl.w  (a1)
; --------------- safe lea end ----------
;
; copy phase
                lea     buffer8,a0
                movea.l screen_adr_fs,a1
                add.l   #SMALL_SCROLLER_OFFSET,a1
                rept    7
                    move.w  (a0),(a1)
add set 8
                    rept    29
                        move.w  add(a0),add(a1)
add set add+8
                    endr
                    lea     SCREEN_WIDTH(a0),a0
                    lea     SCREEN_WIDTH(a1),a1
                endr

                move.w  (a0),(a1)
add set 8
                rept    28
                    move.w  add(a0),add(a1)
add set add+8
                endr
; copy phase end

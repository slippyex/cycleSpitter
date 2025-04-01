                lea     _3dpnt0,a3
                lea     cubeScreenOffsets,a4

                ; preserve the initial screen offset in a5
                movea.l screen_adr_fs,a5
                lea 230*100(a5),a5

                REPT 45
                    movea.l (a3),a2
                    lea     _3dcube,a0
                    adda.l  (a2)+,a0
                    move.l  a5,a1       ; screen initial offset preserved
                    adda.w  (a4)+,a1
; -->
                    REPT 27
                        move.l  (a0)+,(a1)
                        move.l  (a0)+,8(a1)
                        lea     SCREEN_WIDTH(a1),a1
                    ENDR
                    ; exclude the last rept and save the lea
                    move.l  (a0)+,(a1)
                    move.l  (a0)+,8(a1)
; <--
                    move.l  a2,(a3)+
                ENDR





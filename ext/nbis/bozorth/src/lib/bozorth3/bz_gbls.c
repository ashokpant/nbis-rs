/*******************************************************************************

License: 
This software and/or related materials was developed at the National Institute
of Standards and Technology (NIST) by employees of the Federal Government
in the course of their official duties. Pursuant to title 17 Section 105
of the United States Code, this software is not subject to copyright
protection and is in the public domain. 

This software and/or related materials have been determined to be not subject
to the EAR (see Part 734.3 of the EAR for exact details) because it is
a publicly available technology and software, and is freely distributed
to any interested party with no licensing requirements.  Therefore, it is 
permissible to distribute this software as a free download from the internet.

Disclaimer: 
This software and/or related materials was developed to promote biometric
standards and biometric technology testing for the Federal Government
in accordance with the USA PATRIOT Act and the Enhanced Border Security
and Visa Entry Reform Act. Specific hardware and software products identified
in this software were used in order to perform the software development.
In no case does such identification imply recommendation or endorsement
by the National Institute of Standards and Technology, nor does it imply that
the products and equipment identified are necessarily the best available
for the purpose.

This software and/or related materials are provided "AS-IS" without warranty
of any kind including NO WARRANTY OF PERFORMANCE, MERCHANTABILITY,
NO WARRANTY OF NON-INFRINGEMENT OF ANY 3RD PARTY INTELLECTUAL PROPERTY
or FITNESS FOR A PARTICULAR PURPOSE or for any purpose whatsoever, for the
licensed product, however used. In no event shall NIST be liable for any
damages and/or costs, including but not limited to incidental or consequential
damages of any kind, including economic damage or injury to property and lost
profits, regardless of whether NIST shall be advised, have reason to know,
or in fact shall know of the possibility.

By using this software, you agree to bear all risk relating to quality,
use and performance of the software and/or related materials.  You agree
to hold the Government harmless from any claim arising from your use
of the software.

*******************************************************************************/

/***********************************************************************
      LIBRARY: FING - NIST Fingerprint Systems Utilities

      FILE:           BZ_GBLS.C
      ALGORITHM:      Allan S. Bozorth (FBI)
      MODIFICATIONS:  Michael D. Garris (NIST)
                      Stan Janet (NIST)
      DATE:           09/21/2004

      Contains global variables responsible for supporting the
      Bozorth3 fingerprint matching "core" algorithm.

      nbis-rs 0.1.18+: all mutable workspace is BZ_THREAD_LOCAL so
      concurrent Bozorth calls in one process do not race.

***********************************************************************
***********************************************************************/

#include <bozorth.h>

/**************************************************************************/
/* General supporting global variables (thread-local) */
/**************************************************************************/

BZ_THREAD_LOCAL int colp[ COLP_SIZE_1 ][ COLP_SIZE_2 ];
BZ_THREAD_LOCAL int scols[ SCOLS_SIZE_1 ][ COLS_SIZE_2 ];
BZ_THREAD_LOCAL int fcols[ FCOLS_SIZE_1 ][ COLS_SIZE_2 ];
BZ_THREAD_LOCAL int * scolpt[ SCOLPT_SIZE ];
BZ_THREAD_LOCAL int * fcolpt[ FCOLPT_SIZE ];
BZ_THREAD_LOCAL int sc[ SC_SIZE ];

BZ_THREAD_LOCAL int yl[ YL_SIZE_1 ][ YL_SIZE_2 ];


/**************************************************************************/
/* Globals used significantly by sift() */
/**************************************************************************/
#ifdef TARGET_OS
   BZ_THREAD_LOCAL int rq[ RQ_SIZE ];
   BZ_THREAD_LOCAL int tq[ TQ_SIZE ];
   BZ_THREAD_LOCAL int zz[ ZZ_SIZE ];

   BZ_THREAD_LOCAL int rx[ RX_SIZE ];
   BZ_THREAD_LOCAL int mm[ MM_SIZE ];
   BZ_THREAD_LOCAL int nn[ NN_SIZE ];

   BZ_THREAD_LOCAL int qq[ QQ_SIZE ];

   BZ_THREAD_LOCAL int rk[ RK_SIZE ];

   BZ_THREAD_LOCAL int cp[ CP_SIZE ];
   BZ_THREAD_LOCAL int rp[ RP_SIZE ];

   BZ_THREAD_LOCAL int rf[RF_SIZE_1][RF_SIZE_2];
   BZ_THREAD_LOCAL int cf[CF_SIZE_1][CF_SIZE_2];

   BZ_THREAD_LOCAL int y[20000];
#else
   BZ_THREAD_LOCAL int rq[ RQ_SIZE ] = {};
   BZ_THREAD_LOCAL int tq[ TQ_SIZE ] = {};
   BZ_THREAD_LOCAL int zz[ ZZ_SIZE ] = {};

   BZ_THREAD_LOCAL int rx[ RX_SIZE ] = {};
   BZ_THREAD_LOCAL int mm[ MM_SIZE ] = {};
   BZ_THREAD_LOCAL int nn[ NN_SIZE ] = {};

   BZ_THREAD_LOCAL int qq[ QQ_SIZE ] = {};

   BZ_THREAD_LOCAL int rk[ RK_SIZE ] = {};

   BZ_THREAD_LOCAL int cp[ CP_SIZE ] = {};
   BZ_THREAD_LOCAL int rp[ RP_SIZE ] = {};

   BZ_THREAD_LOCAL int rf[RF_SIZE_1][RF_SIZE_2] = {};
   BZ_THREAD_LOCAL int cf[CF_SIZE_1][CF_SIZE_2] = {};

   BZ_THREAD_LOCAL int y[20000] = {};
#endif

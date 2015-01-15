#GapBuffer

This library implements a gapbuffer, a dynamic array in which the unused portion of the array is shifted on insertion & removal. This optimizes for insertions and removals which could occur at any point in the file but tend to occur in localized clusters.

It is currently implemented with a backing RingBuf.

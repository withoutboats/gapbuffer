#GapBuffer

This library implements a gapbuffer, a dynamic array in which the unused portion of the array is shifted on insertion & removal. This optimizes for insertions and removals which could occur at any point in the file.

It currently implements a subset of the methods and traits of a Vec. Eventually, it will hopefully implement all non-deprecated methods and traits of Vec (or similar equivalents, as the case may be) except for push and pop; mutating of the gapbuffer is only provided through the insert and remove methods.

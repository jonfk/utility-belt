#!/usr/bin/env python3

import os

cwd = os.getcwd()
for file in os.listdir(cwd):
    new_file = ""
    for i,c in enumerate(file):
        if (i == 4 or i == 6) and c != '-':
            new_file += '-'
            new_file += c
        elif i == 4 and c == '-':
            break
        else:
            new_file += c
    if len(new_file) > 4:
        print("renaming %(file)s to %(new_file)s" % locals() )
        os.rename(file,new_file)

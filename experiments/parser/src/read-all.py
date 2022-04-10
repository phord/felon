count=1
myfile = open("../core.log-2022040423", "r")
myline = myfile.readline()
while myline:
    count += 1
    myline = myfile.readline()
myfile.close()

print("Lines: ", count)
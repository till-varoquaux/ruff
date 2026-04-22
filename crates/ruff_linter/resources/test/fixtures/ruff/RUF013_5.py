# Annotated shorthand

def f(arg: int @ "metadata" = None):  # RUF013
    pass

def f(arg: (int | None) @ "metadata" = None):
    pass

def f(arg: int @ (Metadata() | Other()) = None): # RUF013
    pass

def f(arg: (int @ "m1") @ "m2" = None): # RUF013
    pass

def f(arg: int @ "m1" @ "m2" = None): # RUF013
    pass

def f(arg: (int | None) @ "m1" @ "m2" = None):
    pass

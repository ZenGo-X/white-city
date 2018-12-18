
COQC = coqc
COQDOC = coqdoc

LIB = subset
SOURCE = relayserver

compile: $(LIB).vo $(SOURCE).vo

doc: $(SOURCE).vo  $(SOURCE).pdf


%.vo: %.v
	$(COQC) $<

$(SOURCE).pdf: $(SOURCE).v $(SOURCE).vo
	$(COQDOC) --pdf --utf8 --parse-comments -t "Kzen : $(SOURCE)"  $<  -o $@ 

clean:
	rm -f *~
	rm -f $(SOURCE).vo
	rm -f $(SOURCE).glob
	rm -f $(LIB).vo
	rm -f $(LIB).glob

cleanall: clean
	rm -f $(SOURCE).pdf


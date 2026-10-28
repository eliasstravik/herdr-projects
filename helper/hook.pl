#!/usr/bin/perl
# Records one Claude Code hook event on a machine Shepherdr runs from the main one: the hook's input as it came, with
# when, one line per event. The main machine reads the file as it grows and cuts each event down (readers/hooks.ts).
# The big parts of a tool call (its input and result) are left out here, except a question's. Put here by Shepherdr.
use strict;
use warnings;
use Time::HiRes ();
use JSON::PP;

my $file = $ARGV[0] or exit 0;
my $in = do { local $/; <STDIN> };
my $e = eval { JSON::PP->new->utf8->decode($in) };
exit 0 unless ref $e eq 'HASH';
delete $e->{tool_response};
delete $e->{tool_input} unless ($e->{tool_name} // '') eq 'AskUserQuestion';
my $line = JSON::PP->new->utf8->canonical->encode({ at => int(Time::HiRes::time() * 1000), raw => $e });
my @st = stat($file);
rename($file, "$file.1") if @st && $st[7] > 1048576;
open(my $fh, '>>', $file) or exit 0;
print $fh "$line\n";
close $fh;

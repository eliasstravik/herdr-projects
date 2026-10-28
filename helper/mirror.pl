#!/usr/bin/perl
# Sends the main machine what the agents' log files here gain, as they grow, so it reads them as its own files
# (mirror.ts). Lines on stdin say what to follow: "D <folder>" every file in it, "F <file>" one file whatever its age,
# "S <ms>" files in folders changed since then, "K <bytes> <file>" how much of it the main machine has already. Out goes, for each piece: "<path>\t<offset>\t<length>\n" and the bytes;
# offset 0 means the file starts over. Put here by Shepherdr.
use strict;
use warnings;
use IO::Select;
use IO::Handle;
use POSIX ();
use Time::HiRes qw(sleep);

binmode STDOUT;
STDOUT->autoflush(0);
my (%dirs, %files, %sent);
my $since = 0;
my $sel = IO::Select->new(\*STDIN);
my $in = '';
# each round the newest files go first, at most PIECE of one file and ROUND in all: a backlog (a machine just added,
# files the main machine never had) goes a round at a time, and what an agent writes now never waits behind it. A
# file that gained at most LIVE goes past ROUND.
my $PIECE = 1 << 20;
my $ROUND = 4 << 20;
my $LIVE = 256 << 10;
# the main machine stopped reading (it started again over a new link): this one goes
my $STUCK = 120;
$SIG{ALRM} = sub { POSIX::_exit(0) };

# a file with something to send: [path, size, mtime]
sub due {
  my ($f, $always) = @_;
  my @s = stat($f);
  return unless @s && -f _;
  my ($ino, $size, $mtime) = ($s[1], $s[7], $s[9]);
  my $k = $sent{$f};
  if (!$k) {
    return unless $always || $mtime * 1000 >= $since;
    $k = $sent{$f} = { ino => $ino, at => 0, fresh => 1 };
  } elsif ($k->{ino} != $ino || $size < $k->{at}) {
    $k->{ino} = $ino;
    $k->{at} = 0;
    $k->{fresh} = 1;
  }
  return if $size <= $k->{at} && !$k->{fresh};
  return [$f, $size, $mtime];
}

# sends at most $max of it from where the main machine is; returns how much went
sub piece {
  my ($f, $size, $max) = @_;
  my $k = $sent{$f};
  my $left = $size - $k->{at};
  return 0 if $left <= 0 && !$k->{fresh};
  open(my $fh, '<', $f) or return 0;
  binmode $fh;
  seek($fh, $k->{at}, 0);
  my $n = read($fh, my $buf, $left > $max ? $max : $left) // 0;
  close $fh;
  print STDOUT "$f\t$k->{at}\t$n\n", $buf;
  $k->{at} += $n;
  $k->{fresh} = 0;
  return $n;
}

while (1) {
  while ($sel->can_read(0)) {
    my $got = sysread(STDIN, my $buf, 65536);
    exit 0 unless $got;
    $in .= $buf;
  }
  while ($in =~ s/^([^\n]*)\n//) {
    my $l = $1;
    if ($l =~ /^D (.+)$/) { $dirs{$1} = 1 }
    elsif ($l =~ /^F (.+)$/) { $files{$1} = 1 }
    elsif ($l =~ /^S (\d+)$/) { $since = $1 }
    elsif ($l =~ /^K (\d+) (.+)$/) {
      # the main machine has this much of it already (from before the link dropped, or before it started again): it
      # goes on from there
      my ($have, $f) = ($1, $2);
      my @s = stat($f);
      $sent{$f} = { ino => $s[1], at => $have, fresh => 0 } if @s && $s[7] >= $have;
    }
  }
  my @due;
  for my $d (keys %dirs) {
    opendir(my $dh, $d) or next;
    my @names = grep { !/^\./ } readdir $dh;
    closedir $dh;
    push @due, grep { defined } map { due("$d/$_", 0) } @names;
  }
  push @due, grep { defined } map { due($_, 1) } keys %files;
  my ($went, $more) = (0, 0);
  alarm $STUCK;
  for my $x (sort { $b->[2] <=> $a->[2] } @due) {
    my ($f, $size) = @$x;
    if ($went >= $ROUND && $size - $sent{$f}{at} > $LIVE) {
      $more = 1;
      next;
    }
    $went += piece($f, $size, $PIECE);
    $more = 1 if $size > $sent{$f}{at};
  }
  STDOUT->flush;
  alarm 0;
  sleep(0.3) unless $more;
}

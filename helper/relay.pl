#!/usr/bin/perl
# Lets a screen watch this machine's shared browser straight from here: agent-browser streams a task's browser on a
# WebSocket on 127.0.0.1 that only takes pages of this machine. This listens on another port of 127.0.0.1 (shared on
# the tailnet by tailscale serve, under a path no one can guess) and passes each connection on to agent-browser's
# port, as from here. Prints the port it listens on; ends when its stdin ends (the main machine let go). Put here by
# Shepherdr.
use strict;
use warnings;
use IO::Socket::INET;
use IO::Select;
use POSIX ':sys_wait_h';

my $to = $ARGV[0] or die "relay.pl <port>\n";
my $l = IO::Socket::INET->new(LocalAddr => '127.0.0.1', LocalPort => 0, Listen => 16, ReuseAddr => 1) or die "listen: $!\n";
$| = 1;
print $l->sockport, "\n";
my %kids;
$SIG{CHLD} = sub { while ((my $p = waitpid(-1, WNOHANG)) > 0) { delete $kids{$p} } };
my $sel = IO::Select->new($l, \*STDIN);
while (1) {
  for my $r ($sel->can_read) {
    if ($r == \*STDIN || fileno($r) == fileno(STDIN)) {
      my $got = sysread(STDIN, my $b, 4096);
      unless ($got) { kill 'TERM', keys %kids; exit 0 }
      next;
    }
    my $c = $l->accept or next;
    my $pid = fork;
    if (!defined $pid) { close $c; next }
    if ($pid) { $kids{$pid} = 1; close $c; next }
    close $l;
    pass($c);
    exit 0;
  }
}

sub pass {
  my ($c) = @_;
  my $head = '';
  while ($head !~ /\r\n\r\n/) {
    my $got = sysread($c, my $b, 8192) or return;
    $head .= $b;
    return if length $head > 65536;
  }
  my ($h, $rest) = split /\r\n\r\n/, $head, 2;
  $h =~ s/^Origin:[^\r\n]*/Origin: http:\/\/127.0.0.1:$to/mi;
  my $u = IO::Socket::INET->new(PeerAddr => '127.0.0.1', PeerPort => $to) or return;
  syswrite($u, "$h\r\n\r\n$rest");
  my $s = IO::Select->new($c, $u);
  while (1) {
    for my $r ($s->can_read) {
      my $got = sysread($r, my $b, 65536);
      return unless $got;
      my $w = $r == $c ? $u : $c;
      my $off = 0;
      while ($off < $got) { my $n = syswrite($w, $b, $got - $off, $off) // return; $off += $n }
    }
  }
}
